// src/core/hooks/useNoticeStats.ts

import { useMemo } from 'react';
import { noticeService } from '@/services/notice';
import type { Notice, NoticeStats } from '@/types/notice';

/**
 * 공지사항 통계 Hook
 */
export function useNoticeStats(notices: Notice[]) {
  const stats = useMemo<NoticeStats>(
    () => noticeService.calculateStatistics(notices),
    [notices]
  );

  const latestDate = useMemo(
    () => noticeService.getLatestNoticeDate(notices),
    [notices]
  );

  return { stats, latestDate };
}
