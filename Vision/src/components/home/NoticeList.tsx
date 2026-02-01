// src/components/home/NoticeList.tsx

import { EmptyState, Spinner } from '@components/ui';

import { NoticeCard } from './NoticeCard';

import type { RefObject } from 'react';

interface Notice {
    link: string;
    campus: string;
    department_name: string;
    department_id: string;
    board_name: string;
    board_id: string;
    title: string;
    date: string | null;
}

interface NoticeListProps {
    notices: Notice[];
    totalCount: number;
    hasMore: boolean;
    loadingMore: boolean;
    loadMoreRef: RefObject<HTMLDivElement | null>;
    onResetFilters: () => void;
}

export function NoticeList({
    notices,
    totalCount,
    hasMore,
    loadingMore,
    loadMoreRef,
    onResetFilters,
}: NoticeListProps) {
    return (
        <section className="list-panel">
            <div className="list-header">
                <div>
                    <div className="list-kicker">Notice Stream</div>
                    <div className="list-title">Notice Feed</div>
                </div>
                <div className="list-meta">
                    <span>
                        {notices.length} / {totalCount} notices
                    </span>
                    <span>Sorted by selected filters</span>
                </div>
            </div>

            {notices.length === 0 ? (
                <EmptyState
                    title="No notices match the criteria"
                    description="Try reducing filters or changing the search query."
                    onReset={onResetFilters}
                    resetLabel="Reset Filters"
                />
            ) : (
                <>
                    <div className="notice-grid">
                        {notices.map((notice, index) => (
                            <NoticeCard
                                key={`${notice.link}-${notice.department_id}-${notice.board_id}-${index}`}
                                notice={notice}
                                index={index}
                            />
                        ))}
                    </div>

                    {/* Infinite Scroll Trigger */}
                    {hasMore && (
                        <div
                            ref={loadMoreRef}
                            style={{
                                minHeight: '100px',
                                display: 'flex',
                                alignItems: 'center',
                                justifyContent: 'center',
                                padding: '2rem',
                            }}
                        >
                            {loadingMore && (
                                <div
                                    style={{
                                        display: 'flex',
                                        flexDirection: 'column',
                                        alignItems: 'center',
                                        gap: '1rem',
                                    }}
                                >
                                    <Spinner />
                                    <p className="notice-kicker">Loading previous notices...</p>
                                </div>
                            )}
                        </div>
                    )}

                    {!hasMore && totalCount > 0 && (
                        <div
                            style={{
                                textAlign: 'center',
                                padding: '2rem',
                                color: 'var(--page-muted)',
                            }}
                        >
                            <p className="notice-kicker">All notices have been loaded.</p>
                        </div>
                    )}
                </>
            )}
        </section>
    );
}
