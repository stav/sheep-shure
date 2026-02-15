import { useEffect } from "react";
import { useNavigate } from "react-router-dom";

export function useKeyboardShortcuts() {
  const navigate = useNavigate();

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      // Only handle ctrl/cmd shortcuts
      if (!e.ctrlKey && !e.metaKey) return;

      switch (e.key) {
        case "n":
          e.preventDefault();
          navigate("/clients/new");
          break;
        case "i":
          e.preventDefault();
          navigate("/import");
          break;
        case ",":
          e.preventDefault();
          navigate("/settings");
          break;
      }
    };

    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [navigate]);
}
