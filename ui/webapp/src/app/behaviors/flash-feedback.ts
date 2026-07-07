import { useState, useRef, useCallback } from "react";

// flashDuration must be lower than or equal to CSS animation duration
export function useFlashFeedback(flashDuration: number = 150) {
  const [flashing, setFlashing] = useState(false);
  const flashTimerRef = useRef<number | null>(null);

  const flash = useCallback(() => {
    if (flashTimerRef.current) {
      clearTimeout(flashTimerRef.current);
    }

    setFlashing(true);

    flashTimerRef.current = window.setTimeout(() => {
      setFlashing(false);
      flashTimerRef.current = null;
    }, flashDuration);
  }, [flashDuration]);

  const clear = useCallback(() => {
    if (flashTimerRef.current) {
      clearTimeout(flashTimerRef.current);
      flashTimerRef.current = null;
    }
    setFlashing(false);
  }, []);

  return {
    flashing,
    flash,
    clear,
  };
}