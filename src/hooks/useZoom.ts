import { useState, useEffect, useCallback, useRef } from "react";

const STEP = 0.1;
const MIN = 0.5;
const MAX = 3.0;
const BROWSER_KEY = "compass-browser-zoom";
const WM_KEY = "compass-wm-zoom";

function clamp(value: number, min: number, max: number) {
  return Math.round(Math.min(max, Math.max(min, value)) * 100) / 100;
}

/**
 * Manages two independent zoom modes:
 *
 * 1. **Browser zoom** (Ctrl +/-, Ctrl+scroll, Ctrl+0 to reset)
 *    CSS `zoom` on <html> — scales content and reflows layout to the new
 *    effective viewport size, exactly like browser zoom.
 *
 * 2. **WM zoom** (Ctrl+Shift +/-, Ctrl+Shift+scroll, Ctrl+Shift+0 to reset)
 *    CSS `transform: scale()` on <body> — magnifies the rendered output
 *    without reflowing layout. Zooms toward the cursor; scroll to pan.
 */
export function useZoom() {
  const [browserZoom, setBrowserZoom] = useState(() =>
    parseFloat(localStorage.getItem(BROWSER_KEY) || "1"),
  );
  const [wmZoom, setWmZoom] = useState(() =>
    parseFloat(localStorage.getItem(WM_KEY) || "1"),
  );

  const prevWmZoomRef = useRef(wmZoom);
  const mouseRef = useRef({
    x: window.innerWidth / 2,
    y: window.innerHeight / 2,
  });

  // ── Browser zoom (CSS zoom on <html>) ──────────────────────────────
  useEffect(() => {
    document.documentElement.style.zoom = String(browserZoom);
    localStorage.setItem(BROWSER_KEY, String(browserZoom));
  }, [browserZoom]);

  // ── WM zoom (CSS transform on <body>, spacer for scroll) ──────────
  useEffect(() => {
    // Spacer is a sibling of <body> inside <html>, positioned absolutely.
    // It gives <html overflow:auto> something to scroll against.
    let spacer = document.getElementById("wm-zoom-spacer");
    if (!spacer) {
      spacer = document.createElement("div");
      spacer.id = "wm-zoom-spacer";
      spacer.style.position = "absolute";
      spacer.style.top = "0";
      spacer.style.left = "0";
      spacer.style.pointerEvents = "none";
      spacer.style.visibility = "hidden";
      document.documentElement.appendChild(spacer);
    }

    const oldZoom = prevWmZoomRef.current;
    prevWmZoomRef.current = wmZoom;

    if (wmZoom === 1) {
      document.body.style.transform = "";
      document.body.style.transformOrigin = "";
      document.body.style.width = "";
      document.body.style.height = "";
      document.documentElement.style.overflow = "";
      document.documentElement.style.position = "";
      spacer.style.display = "none";
    } else {
      document.body.style.transform = `scale(${wmZoom})`;
      document.body.style.transformOrigin = "0 0";
      document.body.style.width = "100vw";
      document.body.style.height = "100vh";
      document.documentElement.style.overflow = "auto";
      document.documentElement.style.position = "relative";
      spacer.style.display = "block";
      spacer.style.width = `${wmZoom * 100}vw`;
      spacer.style.height = `${wmZoom * 100}vh`;

      // Scroll to keep the point under the cursor fixed after zoom change.
      // content_pos = (scroll + cursor) / oldScale
      // newScroll = content_pos * newScale - cursor
      if (oldZoom !== wmZoom) {
        const { x: mx, y: my } = mouseRef.current;
        const sl = oldZoom === 1 ? 0 : document.documentElement.scrollLeft;
        const st = oldZoom === 1 ? 0 : document.documentElement.scrollTop;
        const ratio = wmZoom / (oldZoom === 1 ? 1 : oldZoom);

        requestAnimationFrame(() => {
          document.documentElement.scrollTo(
            (sl + mx) * ratio - mx,
            (st + my) * ratio - my,
          );
        });
      }
    }

    localStorage.setItem(WM_KEY, String(wmZoom));
  }, [wmZoom]);

  // ── Keyboard, wheel & mouse-tracking handlers ─────────────────────
  useEffect(() => {
    const onMouseMove = (e: MouseEvent) => {
      mouseRef.current = { x: e.clientX, y: e.clientY };
    };

    const onKeyDown = (e: KeyboardEvent) => {
      if (!e.ctrlKey && !e.metaKey) return;

      // Use e.code for reliable detection across keyboard layouts
      const isPlus = e.code === "Equal" || e.code === "NumpadAdd";
      const isMinus = e.code === "Minus" || e.code === "NumpadSubtract";
      const isZero = e.code === "Digit0" || e.code === "Numpad0";

      if (!isPlus && !isMinus && !isZero) return;

      e.preventDefault();

      if (e.shiftKey) {
        // WM zoom
        if (isPlus) setWmZoom((p) => clamp(p + STEP, MIN, MAX));
        else if (isMinus) setWmZoom((p) => clamp(p - STEP, MIN, MAX));
        else setWmZoom(1);
      } else {
        // Browser zoom
        if (isPlus) setBrowserZoom((p) => clamp(p + STEP, MIN, MAX));
        else if (isMinus) setBrowserZoom((p) => clamp(p - STEP, MIN, MAX));
        else setBrowserZoom(1);
      }
    };

    const onWheel = (e: WheelEvent) => {
      if (!e.ctrlKey && !e.metaKey) return;
      e.preventDefault();

      // Chromium swaps deltaY→deltaX when Shift is held, so use whichever
      // axis has the value.
      const raw = e.deltaY || e.deltaX;
      const delta = raw > 0 ? -STEP : STEP;

      if (e.shiftKey) {
        setWmZoom((p) => clamp(p + delta, MIN, MAX));
      } else {
        setBrowserZoom((p) => clamp(p + delta, MIN, MAX));
      }
    };

    document.addEventListener("mousemove", onMouseMove);
    document.addEventListener("keydown", onKeyDown);
    document.addEventListener("wheel", onWheel, { passive: false });
    return () => {
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("keydown", onKeyDown);
      document.removeEventListener("wheel", onWheel);
    };
  }, []);

  const resetAll = useCallback(() => {
    setBrowserZoom(1);
    setWmZoom(1);
  }, []);

  return { browserZoom, wmZoom, resetAll };
}
