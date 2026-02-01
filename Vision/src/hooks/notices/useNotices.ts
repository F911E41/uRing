// src/core/hooks/useNotices.ts

import { useEffect, useState, useCallback } from 'react';
import { noticeService } from '@/services/notice';
import type { Notice } from '@/types/notice';

/**
 * 최신 공지사항을 가져오는 Hook
 */
export function useNotices() {
  const [notices, setNotices] = useState<Notice[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);

    const response = await noticeService.getCurrentNotices();

    if (response.status === 'success') {
      setNotices(response.data);
    } else {
      setError(response.message || 'Failed to load notices');
    }

    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return { 
    notices, 
    loading, 
    error, 
    refresh: load 
  };
}
