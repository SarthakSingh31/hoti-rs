use super::ui::{Node, UiUpdater, VideoUI};

pub struct SubtitleManager {
    parts: Vec<(u32, String)>,
}

impl SubtitleManager {
    pub fn new(text: String, total_frames: u32) -> Self {
        let mut utterances = 0;

        for ch in text.chars() {
            match ch {
                '█' => continue,
                '.' | ',' | '?' => utterances += 2,
                _ => utterances += 1,
            }
        }

        let frames_per_ch = total_frames as f64 / utterances as f64;
        let mut prev_utterances = 0;
        let mut current_utterances = 0;
        let mut parts = vec![(0, "".to_owned())];

        for part in text.split(' ') {
            let mut this_utterances = 0;

            for ch in part.chars() {
                match ch {
                    '█' => continue,
                    '.' | ',' | '?' => this_utterances += 2,
                    _ => this_utterances += 1,
                }
            }

            this_utterances += 1;

            if (current_utterances + this_utterances) < 100 {
                let last = &mut parts.last_mut().unwrap().1;
                last.push(' ');
                last.push_str(part);

                current_utterances += this_utterances;
            } else {
                parts.push((
                    ((current_utterances + prev_utterances) as f64 * frames_per_ch).round() as u32,
                    part.to_owned(),
                ));
                prev_utterances += current_utterances;
                current_utterances = this_utterances;
            }
        }

        SubtitleManager { parts }
    }
}

impl UiUpdater for SubtitleManager {
    fn update(&mut self, frame_idx: u32, ui: &mut VideoUI) {
        if let Some((_, s)) = self.parts.iter().find(|(frame, _)| *frame == frame_idx) {
            if let Node::TextCentered { text, .. } = &mut ui.children[3].node {
                *text = s.clone();
            }
        }
    }
}
