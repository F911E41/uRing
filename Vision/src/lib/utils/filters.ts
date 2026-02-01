// src/core/utils/filters.ts

import type { Notice } from '@/types/notice';
import type { FilterOptions } from '@/types/filter';

/**
 * Filter by campus
 */
export function filterByCampus(notices: Notice[], campus: string): Notice[] {
  return notices.filter((notice) => notice.campus === campus);
}

/**
 * Filter by department
 */
export function filterByDepartment(notices: Notice[], departmentName: string): Notice[] {
  return notices.filter((notice) => notice.department_name === departmentName);
}

/**
 * Filter by board
 */
export function filterByBoard(notices: Notice[], boardName: string): Notice[] {
  return notices.filter((notice) => notice.board_name === boardName);
}

/**
 * Search by keyword
 */
export function searchNotices(notices: Notice[], query: string): Notice[] {
  const normalizedQuery = query.trim().toLowerCase();
  if (!normalizedQuery) return notices;

  return notices.filter((notice) =>
    notice.title.toLowerCase().includes(normalizedQuery)
  );
}

/**
 * Apply all filter options
 */
export function applyFilters(notices: Notice[], options: FilterOptions): Notice[] {
  let filtered = notices;

  if (options.campus) {
    filtered = filterByCampus(filtered, options.campus);
  }

  if (options.department) {
    filtered = filterByDepartment(filtered, options.department);
  }

  if (options.board) {
    filtered = filterByBoard(filtered, options.board);
  }

  if (options.searchQuery) {
    filtered = searchNotices(filtered, options.searchQuery);
  }

  return filtered;
}

/**
 * Sort by date (descending)
 */
export function sortByDateDesc(notices: Notice[]): Notice[] {
  return [...notices].sort((a, b) => (b.date || '').localeCompare(a.date || ''));
}

/**
 * Sort by date (ascending)
 */
export function sortByDateAsc(notices: Notice[]): Notice[] {
  return [...notices].sort((a, b) => (a.date || '').localeCompare(b.date || ''));
}

/**
 * Get unique values
 */
export function getUniqueCampuses(notices: Notice[]): string[] {
  return Array.from(new Set(notices.map((n) => n.campus))).sort();
}

export function getUniqueDepartments(notices: Notice[], campus?: string): string[] {
  const filtered = campus ? filterByCampus(notices, campus) : notices;
  return Array.from(new Set(filtered.map((n) => n.department_name))).sort();
}

export function getUniqueBoards(notices: Notice[], campus?: string, department?: string): string[] {
  let filtered = notices;
  if (campus) filtered = filterByCampus(filtered, campus);
  if (department) filtered = filterByDepartment(filtered, department);
  return Array.from(new Set(filtered.map((n) => n.board_name))).sort();
}
