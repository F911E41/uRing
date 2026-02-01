// src/core/hooks/useNoticeFilters.ts

import { useState, useCallback, useMemo } from 'react';
import { noticeService } from '@/services/notice';
import { getUniqueCampuses, getUniqueDepartments, getUniqueBoards } from '@lib/utils/filters';
import type { Notice, FilterOptions } from '@/types/notice';

/**
 * 공지사항 필터링 및 검색 Hook
 */
export function useNoticeFilters(notices: Notice[]) {
  const [selectedCampus, setSelectedCampus] = useState<string | null>(null);
  const [selectedDept, setSelectedDept] = useState<string | null>(null);
  const [selectedBoard, setSelectedBoard] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');

  // 캠퍼스 목록
  const campuses = useMemo(() => getUniqueCampuses(notices), [notices]);

  // 학과 목록 (선택된 캠퍼스 기준)
  const departments = useMemo(
    () => getUniqueDepartments(notices, selectedCampus || undefined),
    [notices, selectedCampus]
  );

  // 게시판 목록 (선택된 캠퍼스/학과 기준)
  const boards = useMemo(
    () => getUniqueBoards(notices, selectedCampus || undefined, selectedDept || undefined),
    [notices, selectedCampus, selectedDept]
  );

  // 필터링된 공지사항
  const filteredNotices = useMemo(() => {
    const options: FilterOptions = {
      campus: selectedCampus,
      department: selectedDept,
      board: selectedBoard,
      searchQuery,
    };
    return noticeService.processNotices(notices, options);
  }, [notices, selectedCampus, selectedDept, selectedBoard, searchQuery]);

  // 필터 초기화
  const resetFilters = useCallback(() => {
    setSelectedCampus(null);
    setSelectedDept(null);
    setSelectedBoard(null);
    setSearchQuery('');
  }, []);

  // 캠퍼스 변경 시 하위 필터 초기화
  const handleCampusChange = useCallback((campus: string | null) => {
    setSelectedCampus(campus);
    setSelectedDept(null);
    setSelectedBoard(null);
  }, []);

  return {
    // 상태
    selectedCampus,
    selectedDept,
    selectedBoard,
    searchQuery,

    // 상태 업데이트
    setSelectedCampus: handleCampusChange,
    setSelectedDept,
    setSelectedBoard,
    setSearchQuery,
    resetFilters,

    // 계산된 값
    campuses,
    departments,
    boards,
    filteredNotices,
  };
}
