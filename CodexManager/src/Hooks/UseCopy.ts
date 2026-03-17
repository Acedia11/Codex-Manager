import { useState, useCallback, useRef } from "react";

export function UseCopy(TimeoutMs = 1500): [boolean, (Text: string) => void] {
  const [Copied, SetCopied] = useState(false);
  const TimerRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  const Copy = useCallback(
    (Text: string) => {
      navigator.clipboard.writeText(Text).catch(() => SetCopied(false));
      SetCopied(true);
      if (TimerRef.current) clearTimeout(TimerRef.current);
      TimerRef.current = setTimeout(() => SetCopied(false), TimeoutMs);
    },
    [TimeoutMs]
  );

  return [Copied, Copy];
}
