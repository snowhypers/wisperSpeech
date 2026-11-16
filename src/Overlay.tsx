import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

type DictationPhase = "idle" | "recording" | "transcribing" | "pasted" | "error";

export default function Overlay() {
  const [phase, setPhase] = useState<DictationPhase>("recording");
  const [level, setLevel] = useState<number>(0.15);
  const [message, setMessage] = useState<string>("Listening...");

  useEffect(() => {
    const unlistenState = listen<{ phase: DictationPhase; message?: string }>(
      "dictation-state",
      (event) => {
        setPhase(event.payload.phase);
        if (event.payload.message) {
          setMessage(event.payload.message);
        } else if (event.payload.phase === "recording") {
          setMessage("Listening...");
        } else if (event.payload.phase === "transcribing") {
          setMessage("Whistle STT...");
        } else if (event.payload.phase === "pasted") {
          setMessage("Pasted!");
        }
      }
    );

    const unlistenLevel = listen<number>("audio-level", (event) => {
      setLevel(Math.min(1, Math.max(0.05, event.payload)));
    });

    return () => {
      unlistenState.then((f) => f());
      unlistenLevel.then((f) => f());
    };
  }, []);

  const bars = [0.4, 0.75, 1.0, 0.65, 0.9, 0.5, 0.85, 0.6, 0.95, 0.45];

  return (
    <div className="overlay-wrap">
      <div className="overlay-pill" data-tauri-drag-region>
        <div className="overlay-status">
          {phase === "recording" && <span className="pulse" />}
          {phase === "transcribing" && (
            <span className="dot" style={{ background: "#38bdf8" }} />
          )}
          {phase === "pasted" && (
            <span className="dot" style={{ background: "#10b981" }} />
          )}
          {phase === "error" && (
            <span className="dot" style={{ background: "#ef4444" }} />
          )}
          <span>{message}</span>
        </div>

        <div className="waveform">
          {bars.map((mult, idx) => {
            const h =
              phase === "recording"
                ? Math.max(4, Math.round(level * mult * 24))
                : phase === "transcribing"
                ? 8 + ((idx * 3) % 10)
                : 4;
            return (
              <div
                key={idx}
                className="wave-bar"
                style={{ height: `${h}px` }}
              />
            );
          })}
        </div>
      </div>
    </div>
  );
}
