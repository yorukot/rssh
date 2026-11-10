use crate::input::local_commands::LocalCmd;

const PREFIX_BYTE: u8 = 0x1D; // Ctrl+]
const ESC: u8 = 0x1B;

// Bracketed paste markers
const BP_START: &[u8] = b"\x1b[200~";
const BP_END: &[u8] = b"\x1b[201~";

#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Forward(Vec<u8>),
    LocalCommand(LocalCmd),
    /// Content received via bracketed paste (drag-and-drop or terminal paste).
    BracketedPaste(Vec<u8>),
    Pending,
}

pub struct InputDecoder {
    ctrl_state: CtrlState,
    paste_buf: Option<Vec<u8>>,
    // Partial sequence at end of a chunk that might be start/end of BP markers
    tail: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CtrlState {
    Normal,
    PrefixReceived,
}

impl InputDecoder {
    pub fn new() -> Self {
        Self {
            ctrl_state: CtrlState::Normal,
            paste_buf: None,
            tail: Vec::new(),
        }
    }

    pub fn feed_bytes(&mut self, input: &[u8]) -> Vec<Action> {
        // Prepend any leftover tail from previous call
        let bytes = if self.tail.is_empty() {
            input.to_vec()
        } else {
            let mut combined = std::mem::take(&mut self.tail);
            combined.extend_from_slice(input);
            combined
        };

        let mut actions = Vec::new();
        let mut i = 0;

        while i < bytes.len() {
            if self.paste_buf.is_some() {
                // In bracketed paste mode: look for BP_END
                if let Some(end_pos) = find_subsequence(&bytes[i..], BP_END) {
                    // Found end of paste
                    let paste_content_end = i + end_pos;
                    self.paste_buf
                        .as_mut()
                        .unwrap()
                        .extend_from_slice(&bytes[i..paste_content_end]);
                    let content = self.paste_buf.take().unwrap();
                    actions.push(Action::BracketedPaste(content));
                    i = paste_content_end + BP_END.len();
                } else {
                    // Check if the end of the buffer could be the start of BP_END
                    let safe_end = safe_consume_end(&bytes[i..], BP_END);
                    self.paste_buf
                        .as_mut()
                        .unwrap()
                        .extend_from_slice(&bytes[i..i + safe_end]);
                    if safe_end < bytes.len() - i {
                        self.tail = bytes[i + safe_end..].to_vec();
                    }
                    break;
                }
            } else {
                // Normal mode: look for BP_START
                if bytes[i] == ESC {
                    if let Some(start_pos) = find_subsequence(&bytes[i..], BP_START) {
                        if start_pos == 0 {
                            // BP_START found at current position
                            self.paste_buf = Some(Vec::new());
                            i += BP_START.len();
                            continue;
                        }
                    }
                    // Check if end of buffer might be a partial BP_START
                    let remaining = &bytes[i..];
                    if remaining.len() < BP_START.len() && BP_START.starts_with(remaining) {
                        self.tail = remaining.to_vec();
                        break;
                    }
                }

                // Process through Ctrl+] decoder
                let action = self.feed_ctrl(bytes[i]);
                match action {
                    Action::Pending => {}
                    Action::Forward(ref data) => {
                        if let Some(Action::Forward(existing)) = actions.last_mut() {
                            existing.extend_from_slice(data);
                        } else {
                            actions.push(action);
                        }
                    }
                    _ => actions.push(action),
                }
                i += 1;
            }
        }

        if self.ctrl_state == CtrlState::PrefixReceived && self.paste_buf.is_none() {
            actions.push(Action::Pending);
        }

        actions
    }

    fn feed_ctrl(&mut self, byte: u8) -> Action {
        match self.ctrl_state {
            CtrlState::Normal => {
                if byte == PREFIX_BYTE {
                    self.ctrl_state = CtrlState::PrefixReceived;
                    Action::Pending
                } else {
                    Action::Forward(vec![byte])
                }
            }
            CtrlState::PrefixReceived => {
                self.ctrl_state = CtrlState::Normal;
                match byte {
                    b'v' => Action::LocalCommand(LocalCmd::UploadClipboard),
                    b's' => Action::LocalCommand(LocalCmd::ShowStatus),
                    b'd' => Action::LocalCommand(LocalCmd::Discard),
                    PREFIX_BYTE => Action::Forward(vec![PREFIX_BYTE]),
                    other => Action::Forward(vec![PREFIX_BYTE, other]),
                }
            }
        }
    }

    pub fn flush(&mut self) -> Option<Action> {
        if self.ctrl_state == CtrlState::PrefixReceived {
            self.ctrl_state = CtrlState::Normal;
            Some(Action::Forward(vec![PREFIX_BYTE]))
        } else if !self.tail.is_empty() {
            let tail = std::mem::take(&mut self.tail);
            Some(Action::Forward(tail))
        } else {
            None
        }
    }

    pub fn is_pending(&self) -> bool {
        self.ctrl_state == CtrlState::PrefixReceived
    }
}

impl Default for InputDecoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Find the position of `needle` in `haystack`.
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|w| w == needle)
}

/// How many bytes from `data` can be safely consumed without risking
/// splitting a `marker` that might start near the end.
fn safe_consume_end(data: &[u8], marker: &[u8]) -> usize {
    if data.len() < marker.len() {
        // Could be partial marker
        if marker.starts_with(data) {
            return 0;
        }
        return data.len();
    }
    // Check if the tail of data could be a prefix of marker
    for start in (data.len().saturating_sub(marker.len() - 1))..data.len() {
        let tail = &data[start..];
        if marker.starts_with(tail) {
            return start;
        }
    }
    data.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_bytes_forward() {
        let mut dec = InputDecoder::new();
        let actions = dec.feed_bytes(b"hello");
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0], Action::Forward(b"hello".to_vec()));
    }

    #[test]
    fn ctrl_prefix_then_v() {
        let mut dec = InputDecoder::new();
        let actions = dec.feed_bytes(&[PREFIX_BYTE, b'v']);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0], Action::LocalCommand(LocalCmd::UploadClipboard));
    }

    #[test]
    fn ctrl_prefix_then_s() {
        let mut dec = InputDecoder::new();
        let actions = dec.feed_bytes(&[PREFIX_BYTE, b's']);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0], Action::LocalCommand(LocalCmd::ShowStatus));
    }

    #[test]
    fn double_prefix_sends_literal() {
        let mut dec = InputDecoder::new();
        let actions = dec.feed_bytes(&[PREFIX_BYTE, PREFIX_BYTE]);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0], Action::Forward(vec![PREFIX_BYTE]));
    }

    #[test]
    fn prefix_then_unknown() {
        let mut dec = InputDecoder::new();
        let actions = dec.feed_bytes(&[PREFIX_BYTE, b'z']);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0], Action::Forward(vec![PREFIX_BYTE, b'z']));
    }

    #[test]
    fn bracketed_paste_detected() {
        let mut dec = InputDecoder::new();
        let mut input = Vec::new();
        input.extend_from_slice(BP_START);
        input.extend_from_slice(b"/Users/test/image.png");
        input.extend_from_slice(BP_END);
        let actions = dec.feed_bytes(&input);
        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0],
            Action::BracketedPaste(b"/Users/test/image.png".to_vec())
        );
    }

    #[test]
    fn bracketed_paste_with_surrounding_text() {
        let mut dec = InputDecoder::new();
        let mut input = Vec::new();
        input.extend_from_slice(b"before");
        input.extend_from_slice(BP_START);
        input.extend_from_slice(b"paste content");
        input.extend_from_slice(BP_END);
        input.extend_from_slice(b"after");
        let actions = dec.feed_bytes(&input);
        assert_eq!(actions.len(), 3);
        assert_eq!(actions[0], Action::Forward(b"before".to_vec()));
        assert_eq!(
            actions[1],
            Action::BracketedPaste(b"paste content".to_vec())
        );
        assert_eq!(actions[2], Action::Forward(b"after".to_vec()));
    }

    #[test]
    fn bracketed_paste_split_across_calls() {
        let mut dec = InputDecoder::new();

        // First chunk: start marker + partial content
        let mut chunk1 = Vec::new();
        chunk1.extend_from_slice(BP_START);
        chunk1.extend_from_slice(b"/path/to/file");
        let actions1 = dec.feed_bytes(&chunk1);
        assert!(actions1.is_empty()); // Still accumulating

        // Second chunk: rest + end marker
        let mut chunk2 = Vec::new();
        chunk2.extend_from_slice(b".png");
        chunk2.extend_from_slice(BP_END);
        let actions2 = dec.feed_bytes(&chunk2);
        assert_eq!(actions2.len(), 1);
        assert_eq!(
            actions2[0],
            Action::BracketedPaste(b"/path/to/file.png".to_vec())
        );
    }

    #[test]
    fn non_paste_escape_sequences_forwarded() {
        let mut dec = InputDecoder::new();
        // A regular CSI sequence (not bracketed paste)
        let input = b"\x1b[1;2H";
        let actions = dec.feed_bytes(input);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0], Action::Forward(input.to_vec()));
    }

    #[test]
    fn ctrl_v_during_paste_mode_not_intercepted() {
        let mut dec = InputDecoder::new();
        let mut input = Vec::new();
        input.extend_from_slice(BP_START);
        input.push(PREFIX_BYTE);
        input.push(b'v');
        input.extend_from_slice(BP_END);
        let actions = dec.feed_bytes(&input);
        assert_eq!(actions.len(), 1);
        // Ctrl+] v inside paste is treated as paste content, not a command
        assert_eq!(
            actions[0],
            Action::BracketedPaste(vec![PREFIX_BYTE, b'v'])
        );
    }

    #[test]
    fn prefix_at_end_is_pending() {
        let mut dec = InputDecoder::new();
        let actions = dec.feed_bytes(&[b'a', PREFIX_BYTE]);
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0], Action::Forward(vec![b'a']));
        assert_eq!(actions[1], Action::Pending);
    }

    #[test]
    fn flush_pending_prefix() {
        let mut dec = InputDecoder::new();
        dec.feed_bytes(&[PREFIX_BYTE]);
        let flushed = dec.flush();
        assert_eq!(flushed, Some(Action::Forward(vec![PREFIX_BYTE])));
    }

    #[test]
    fn empty_input() {
        let mut dec = InputDecoder::new();
        let actions = dec.feed_bytes(&[]);
        assert!(actions.is_empty());
    }

    #[test]
    fn binary_data_passthrough() {
        let mut dec = InputDecoder::new();
        let input: Vec<u8> = (0u8..=255)
            .filter(|&b| b != PREFIX_BYTE && b != ESC)
            .collect();
        let actions = dec.feed_bytes(&input);
        assert_eq!(actions.len(), 1);
        if let Action::Forward(ref data) = actions[0] {
            assert_eq!(data, &input);
        } else {
            panic!("expected Forward");
        }
    }
}
