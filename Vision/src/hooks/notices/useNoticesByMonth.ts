// src/core/hooks/useNoticesByMonth.ts

import { useEffect, useState, useCallback } from 'react';
import { noticeService } from '@/services/notice';
import type { Notice } from '@/types/notice';

/**
 * 특정 월의 공지사항을 가져오는 Hook
 */
export function useNoticesByMonth(year: number, month: number) {
  const [notices, setNotices] = useState<Notice[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);

    const response = await noticeService.getNoticesByMonth(year, month);

    if (response.status === 'success') {
      setNotices(response.data);
    } else {
      setError(response.message || 'Failed to load notices');
    }

    setLoading(false);
  }, [year, month]);

  useEffect(() => {
    void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [year, month]);

  return { 
    notices, 
    loading, 
    error, 
    refresh: load 
  };
}
