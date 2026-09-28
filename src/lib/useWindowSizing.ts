import { useEffect, type RefObject } from "react";
import { api } from "./api";

export function useWindowSizing(ref: RefObject<HTMLElement | null>, ready: boolean): void {
  useEffect(() => {
    const element = ref.current;
    if (!ready || !element) return;
    let presented = false;
    let frame = 0;
    let lastHeight = 0;

    const sync = (): void => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        const height = Math.ceil(element.getBoundingClientRect().height);
        if (height === lastHeight) return;
        lastHeight = height;
        if (presented) {
          void api.resize(height);
          return;
        }
        presented = true;
        void api.present(height);
      });
    };

    const observer = new ResizeObserver(sync);
    void document.fonts.ready.then(() => observer.observe(element));
    return () => {
      observer.disconnect();
      cancelAnimationFrame(frame);
    };
  }, [ref, ready]);
}
