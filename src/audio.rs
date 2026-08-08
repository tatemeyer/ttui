//! Optional sound-effect playback hook — apps depend on `AudioSink`
//! rather than a concrete audio backend so they stay testable/silent
//! by default.

/// Plays a named sound-effect event; implementations pick the backend
/// (or do nothing, like `NullAudioSink`).
pub trait AudioSink {
    /// Trigger the sound effect associated with `event_id`.
    fn play(&mut self, event_id: &str);
}

/// An `AudioSink` that discards every event — the default for apps
/// that don't need real audio.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct NullAudioSink;

impl AudioSink for NullAudioSink {
    fn play(&mut self, _event_id: &str) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_audio_sink_play_does_not_panic() {
        let mut sink = NullAudioSink;
        sink.play("test_event");
        // Test passes if no panic occurs
    }

    #[test]
    fn test_audio_sink_trait_is_implementable_and_records_calls() {
        struct RecordingAudioSink {
            calls: Vec<String>,
        }

        impl AudioSink for RecordingAudioSink {
            fn play(&mut self, event_id: &str) {
                self.calls.push(event_id.to_string());
            }
        }

        let mut sink = RecordingAudioSink { calls: Vec::new() };

        sink.play("event_1");
        sink.play("event_2");

        assert_eq!(sink.calls.len(), 2);
        assert_eq!(sink.calls[0], "event_1");
        assert_eq!(sink.calls[1], "event_2");
    }

    #[test]
    fn test_audio_sink_trait_object_safety() {
        let mut sink: Box<dyn AudioSink> = Box::new(NullAudioSink);
        sink.play("test_event");
        // Test passes if trait object compiles and can be called
    }
}
