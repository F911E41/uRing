// src/app/page.tsx

'use client';

import { useInfiniteNotices, useNoticeFilters, useNoticeStats, useIntersectionObserver } from '@hooks/index';

import { HeroSection, FilterSidebar, NoticeList } from '@components/home';
import { LoadingState, ErrorState } from '@components/ui';

// @TODO: This label should be dynamic based on the data source used.
const dataSourceLabel = 'Local Snapshot (public/v1)';

export default function Home() {
  // Infinite Notices Hook
  const {
    notices,
    loading,
    loadingMore,
    error,
    hasMore,
    loadMore
  } = useInfiniteNotices();

  // Filter Hook and State Management
  const {
    selectedCampus,
    selectedDept,
    selectedBoard,
    searchQuery,
    setSelectedCampus,
    setSelectedDept,
    setSelectedBoard,
    setSearchQuery,
    resetFilters,
    campuses,
    departments,
    boards,
    filteredNotices,
  } = useNoticeFilters(notices);

  // Notice Statistics
  const { stats, latestDate } = useNoticeStats(notices);

  // Infinite Scroll Observer
  const loadMoreRef = useIntersectionObserver(
    () => {
      if (!loadingMore && hasMore) {
        loadMore();
      }
    },
    { enabled: !loading && !loadingMore && hasMore }
  );

  // Loading State
  if (loading) {
    return <LoadingState />;
  }

  // Error State
  if (error) {
    return <ErrorState message={error} />;
  }

  // Main Render
  return (
    <div className="app-shell">
      <HeroSection
        dataSourceLabel={dataSourceLabel}
        latestDate={latestDate}
        totalNotices={notices.length}
        filteredCount={filteredNotices.length}
        stats={stats}
      />

      <main className="main-grid">
        <FilterSidebar
          searchQuery={searchQuery}
          onSearchChange={setSearchQuery}
          selectedCampus={selectedCampus}
          selectedDept={selectedDept}
          selectedBoard={selectedBoard}
          onCampusChange={setSelectedCampus}
          onDeptChange={setSelectedDept}
          onBoardChange={setSelectedBoard}
          campuses={campuses}
          departments={departments}
          boards={boards}
          onResetFilters={resetFilters}
        />

        <NoticeList
          notices={filteredNotices}
          totalCount={notices.length}
          hasMore={hasMore}
          loadingMore={loadingMore}
          loadMoreRef={loadMoreRef}
          onResetFilters={resetFilters}
        />
      </main>

      <footer className="footer">
        <p>uRing Viewer - Yonsei University Notice Monitor</p>
        <p>Provides a unified feed by refining campus notice data.</p>
      </footer>
    </div>
  );
}
