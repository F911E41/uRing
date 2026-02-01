// src/core/hooks/useInfiniteNotices.ts

import { useState, useEffect, useCallback, useRef } from 'react';

import { noticeService } from '@/services/notice';

import type { Notice } from '@/types/notice';

/**
 * 무한 스크롤을 위한 공지사항 로딩 Hook
 */
export function useInfiniteNotices() {
  const [notices, setNotices] = useState<Notice[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [hasMore, setHasMore] = useState(true);

  // 현재 로드된 가장 오래된 날짜 추적
  const [oldestDate, setOldestDate] = useState<Date | null>(null);

  // 중복 로딩 방지
  const isLoadingRef = useRef(false);

  /**
   * 날짜에서 년/월 추출
   */
  const getYearMonth = (date: Date): { year: number; month: number } => {
    return {
      year: date.getFullYear(),
      month: date.getMonth() + 1,
    };
  };

  /**
   * 한 달 이전 날짜 계산
   */
  const getPreviousMonth = (date: Date): Date => {
    const newDate = new Date(date);
    newDate.setMonth(newDate.getMonth() - 1);
    return newDate;
  };

  /**
   * 최신 공지사항 로드
   */
  const loadInitial = useCallback(async () => {
    if (isLoadingRef.current) return;

    isLoadingRef.current = true;
    setLoading(true);
    setError(null);

    const response = await noticeService.getCurrentNotices();

    if (response.status === 'success') {
      setNotices(response.data);

      // 가장 오래된 날짜 찾기
      if (response.data.length > 0) {
        const dates = response.data
          .map(n => n.date)
          .filter(Boolean)
          .sort();

        if (dates.length > 0) {
          const oldest = dates[0]; // YYYY.MM.DD 형식
          const [year, month] = oldest.split('.').map(Number);
          setOldestDate(new Date(year, month - 1, 1));
        }
      }
    } else {
      setError(response.message || 'Failed to load notices');
      setHasMore(false);
    }

    setLoading(false);
    isLoadingRef.current = false;
  }, []);

  /**
   * 이전 달 공지사항 로드
   */
  const loadMore = useCallback(async () => {
    if (isLoadingRef.current || !hasMore || !oldestDate) return;

    isLoadingRef.current = true;
    setLoadingMore(true);

    try {
      // 한 달 이전으로 이동
      const previousMonth = getPreviousMonth(oldestDate);
      const { year, month } = getYearMonth(previousMonth);

      const response = await noticeService.getNoticesByMonth(year, month);

      if (response.status === 'success' && response.data.length > 0) {
        // 기존 공지사항에 추가 (중복 제거)
        setNotices(prev => {
          const existingIds = new Set(prev.map(n => `${n.link}-${n.date}`));
          const newNotices = response.data.filter(
            n => !existingIds.has(`${n.link}-${n.date}`)
          );
          return [...prev, ...newNotices];
        });

        // 다음 로드를 위해 날짜 업데이트
        setOldestDate(previousMonth);
      } else {
        // 더 이상 로드할 데이터 없음
        setHasMore(false);
      }
    } catch (err) {
      console.error('Failed to load more notices:', err);
      setHasMore(false);
    } finally {
      setLoadingMore(false);
      isLoadingRef.current = false;
    }
  }, [hasMore, oldestDate]);

  // 초기 로드
  useEffect(() => {
    loadInitial();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return {
    notices,
    loading,
    loadingMore,
    error,
    hasMore,
    loadMore,
    refresh: loadInitial,
  };
}
