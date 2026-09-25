"use client";

import { useEffect } from "react";

export default function ScrollReveal() {
  useEffect(() => {
    const elements = Array.from(
      document.querySelectorAll<HTMLElement>("[data-reveal]")
    );

    if (!elements.length) {
      return;
    }

    if (
      window.matchMedia("(prefers-reduced-motion: reduce)").matches
    ) {
      elements.forEach((element) => {
        element.dataset.visible = "true";
      });

      return;
    }

    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (!entry.isIntersecting) {
            return;
          }

          const element = entry.target as HTMLElement;
          element.dataset.visible = "true";

          observer.unobserve(element);
        });
      },
      {
        threshold: 0.12,
        rootMargin: "0px 0px -50px 0px",
      }
    );

    elements.forEach((element) => observer.observe(element));

    return () => observer.disconnect();
  }, []);

  return null;
}
