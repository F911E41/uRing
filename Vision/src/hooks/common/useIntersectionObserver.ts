// src/core/hooks/useIntersectionObserver.ts

import { useEffect, useRef, RefObject } from 'react';

interface UseIntersectionObserverOptions {
  threshold?: number;
  root?: Element | null;
  rootMargin?: string;
  enabled?: boolean;
}

/**
 * Intersection Observer Hook
 * Executes callback when the element becomes visible on the screen
 */
export function useIntersectionObserver(
  callback: () => void,
  options: UseIntersectionObserverOptions = {}
): RefObject<HTMLDivElement | null> {
  const {
    threshold = 0.1,
    root = null,
    rootMargin = '100px',
    enabled = true,
  } = options;

  const targetRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!enabled) return;

    const target = targetRef.current;
    if (!target) return;

    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            callback();
          }
        });
      },
      {
        threshold,
        root,
        rootMargin,
      }
    );

    observer.observe(target);

    return () => {
      if (target) {
        observer.unobserve(target);
      }
    };
  }, [callback, threshold, root, rootMargin, enabled]);

  return targetRef;
}
