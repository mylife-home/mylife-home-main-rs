import { useRef, useCallback, useState } from "react";

export function useClickActions(
  onSingleClick: () => void,
  onDoubleClick: () => void,
  delay: number = 300 // ms threshold between clicks
) {
  const [active, setActive] = useState(false);
  const timerRef = useRef<number | null>(null);
  const clickCountRef = useRef(0);

  const start = useCallback((e: React.SyntheticEvent) => {
    e.preventDefault();
    setActive(true);
  }, []);

  const stop = useCallback((e: React.SyntheticEvent) => {
    e.preventDefault();
    setActive(false);

    clickCountRef.current += 1;

    switch (clickCountRef.current) {

      case 1:
      // First click - start timer for single click
      timerRef.current = window.setTimeout(() => {
        onSingleClick();
        clickCountRef.current = 0;
        timerRef.current = null;
      }, delay);

      break;

    case 2:
      // Second click - cancel timer and execute double click
      if (timerRef.current) {
        clearTimeout(timerRef.current);
        timerRef.current = null;
      }

      onDoubleClick();

      clickCountRef.current = 0;

      break;
    }
  }, [onSingleClick, onDoubleClick, delay]);

  const cancel = useCallback((e: React.SyntheticEvent) => {
    e.preventDefault();
    setActive(false);
    
    // Cancel everything
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }

    if (clickCountRef.current === 1) {
      // If we had a single click pending, execute it now
      onSingleClick();
    }

    clickCountRef.current = 0;
  }, []);

  return { 
    active,
    start,
    stop,
    cancel,
  };
}
