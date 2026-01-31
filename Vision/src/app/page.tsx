// src/app/page.tsx

'use client';

import { useEffect, useMemo, useState } from 'react';
import { fetchAllNotices, type Notice } from '@core/network/api';

const dataSourceLabel = process.env.NEXT_PUBLIC_S3_BASE_URL
  ? 'S3 live feed'
  : process.env.NEXT_PUBLIC_NOTICES_URL || process.env.NEXT_PUBLIC_NOTICES_BASE_URL
    ? 'Remote snapshot'
    : 'Local snapshot';

export default function Home() {
  const [notices, setNotices] = useState<Notice[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedCampus, setSelectedCampus] = useState<string | null>(null);
  const [selectedDept, setSelectedDept] = useState<string | null>(null);
  const [selectedBoard, setSelectedBoard] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');

  const campuses = useMemo(
    () => Array.from(new Set(notices.map((notice) => notice.campus))).sort(),
    [notices]
  );

  const departments = useMemo(() => {
    if (!selectedCampus) return [];
    return Array.from(
      new Set(
        notices
          .filter((notice) => notice.campus === selectedCampus)
          .map((notice) => notice.department_name)
      )
    ).sort();
  }, [notices, selectedCampus]);

  const boards = useMemo(() => {
    if (!selectedCampus) return [];
    return Array.from(
      new Set(
        notices
          .filter((notice) => notice.campus === selectedCampus)
          .filter(
            (notice) =>
              !selectedDept || notice.department_name === selectedDept
          )
          .map((notice) => notice.board_name)
      )
    ).sort();
  }, [notices, selectedCampus, selectedDept]);

  const filteredNotices = useMemo(() => {
    const normalizedQuery = searchQuery.trim().toLowerCase();
    return notices.filter((notice) => {
      if (selectedCampus && notice.campus !== selectedCampus) return false;
      if (selectedDept && notice.department_name !== selectedDept) return false;
      if (selectedBoard && notice.board_name !== selectedBoard) return false;
      if (normalizedQuery && !notice.title.toLowerCase().includes(normalizedQuery)) {
        return false;
      }
      return true;
    });
  }, [notices, searchQuery, selectedBoard, selectedCampus, selectedDept]);

  const sortedNotices = useMemo(() => {
    return [...filteredNotices].sort((a, b) =>
      (b.date || '').localeCompare(a.date || '')
    );
  }, [filteredNotices]);

  const stats = useMemo(() => {
    return {
      campuses: new Set(notices.map((notice) => notice.campus)).size,
      departments: new Set(notices.map((notice) => notice.department_name)).size,
      boards: new Set(notices.map((notice) => notice.board_name)).size,
    };
  }, [notices]);

  const latestDate = useMemo(() => {
    return notices.reduce((latest, notice) => {
      if (!notice.date) return latest;
      return notice.date > latest ? notice.date : latest;
    }, '');
  }, [notices]);

  useEffect(() => {
    const loadData = async () => {
      setLoading(true);
      setError(null);

      const response = await fetchAllNotices();

      if (response.status === 'success') {
        setNotices(response.data);
      } else {
        setError(response.message || 'Failed to load notices');
      }

      setLoading(false);
    };

    loadData();
  }, []);

  const handleCampusChange = (campus: string | null) => {
    setSelectedCampus(campus);
    setSelectedDept(null);
    setSelectedBoard(null);
  };

  const resetFilters = () => {
    handleCampusChange(null);
    setSelectedDept(null);
    setSelectedBoard(null);
    setSearchQuery('');
  };

  if (loading) {
    return (
      <div className="state-screen">
        <div className="state-card">
          <div className="spinner" />
          <h2 className="panel-title">공지사항을 불러오는 중</h2>
          <p className="hero-subtitle">캠퍼스별 최신 공지를 확인하고 있습니다.</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="state-screen">
        <div className="state-card">
          <h2 className="panel-title">데이터를 불러올 수 없습니다</h2>
          <p className="hero-subtitle">{error}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="app-shell">
      <header className="hero">
        <div className="hero-inner">
          <div className="brand-row">
            <div className="brand-lockup">
              <div className="brand-mark">uR</div>
              <div>
                <div className="brand-tag">uRing Notice Desk</div>
                <h1 className="hero-title">연세대학교 공지 뷰어</h1>
                <p className="hero-subtitle">
                  캠퍼스, 학과, 게시판을 넘나드는 공지를 10분 단위로
                  끌어와 실시간에 가깝게 정리.
                </p>
              </div>
            </div>
            <div className="hero-meta">
              <div className="meta-pill">Data: {dataSourceLabel}</div>
              <div className="meta-pill">
                Latest: {latestDate || 'no data'}
              </div>
            </div>
          </div>
          <div className="hero-stats">
            <div className="stat-card">
              <div className="stat-label">Notices</div>
              <div className="stat-value">{notices.length}</div>
            </div>
            <div className="stat-card">
              <div className="stat-label">Campuses</div>
              <div className="stat-value">{stats.campuses}</div>
            </div>
            <div className="stat-card">
              <div className="stat-label">Departments</div>
              <div className="stat-value">{stats.departments}</div>
            </div>
            <div className="stat-card">
              <div className="stat-label">Boards</div>
              <div className="stat-value">{stats.boards}</div>
            </div>
          </div>
        </div>
      </header>

      <main className="main-grid">
        <aside className="side-panel">
          <div className="panel-card">
            <div className="panel-title">빠른 검색</div>
            <div className="input-wrap">
              <input
                className="text-input"
                type="text"
                value={searchQuery}
                onChange={(event) => setSearchQuery(event.target.value)}
                placeholder="제목 또는 키워드를 입력하세요"
              />
              {searchQuery && (
                <button
                  type="button"
                  className="input-clear"
                  aria-label="검색어 지우기"
                  onClick={() => setSearchQuery('')}
                >
                  x
                </button>
              )}
            </div>
          </div>

          <div className="panel-card">
            <div className="panel-title">필터 선택</div>
            <div className="space-y-4">
              <div>
                <label className="notice-kicker" htmlFor="campus-select">
                  캠퍼스
                </label>
                <select
                  id="campus-select"
                  className="select-input"
                  value={selectedCampus || ''}
                  onChange={(event) =>
                    handleCampusChange(event.target.value || null)
                  }
                >
                  <option value="">전체 캠퍼스</option>
                  {campuses.map((campus) => (
                    <option key={campus} value={campus}>
                      {campus}
                    </option>
                  ))}
                </select>
              </div>

              <div>
                <label className="notice-kicker" htmlFor="dept-select">
                  학부 / 학과
                </label>
                <select
                  id="dept-select"
                  className="select-input"
                  value={selectedDept || ''}
                  onChange={(event) => setSelectedDept(event.target.value || null)}
                  disabled={!selectedCampus}
                >
                  <option value="">전체 학부</option>
                  {departments.map((dept) => (
                    <option key={dept} value={dept}>
                      {dept}
                    </option>
                  ))}
                </select>
              </div>

              <div>
                <label className="notice-kicker" htmlFor="board-select">
                  게시판
                </label>
                <select
                  id="board-select"
                  className="select-input"
                  value={selectedBoard || ''}
                  onChange={(event) => setSelectedBoard(event.target.value || null)}
                  disabled={!selectedCampus}
                >
                  <option value="">전체 게시판</option>
                  {boards.map((board) => (
                    <option key={board} value={board}>
                      {board}
                    </option>
                  ))}
                </select>
              </div>
            </div>
          </div>

          <div className="panel-card">
            <div className="panel-title">활성 필터</div>
            <div className="chip-row">
              {selectedCampus && (
                <span className="chip">
                  {selectedCampus}
                  <button onClick={() => handleCampusChange(null)}>x</button>
                </span>
              )}
              {selectedDept && (
                <span className="chip">
                  {selectedDept}
                  <button onClick={() => setSelectedDept(null)}>x</button>
                </span>
              )}
              {selectedBoard && (
                <span className="chip">
                  {selectedBoard}
                  <button onClick={() => setSelectedBoard(null)}>x</button>
                </span>
              )}
              {searchQuery && (
                <span className="chip">
                  {`"${searchQuery}"`}
                  <button onClick={() => setSearchQuery('')}>x</button>
                </span>
              )}
            </div>
            {(selectedCampus || selectedDept || selectedBoard || searchQuery) && (
              <button type="button" className="reset-link" onClick={resetFilters}>
                전체 초기화
              </button>
            )}
          </div>
        </aside>

        <section className="list-panel">
          <div className="list-header">
            <div>
              <div className="list-kicker">Notice Stream</div>
              <div className="list-title">공지사항 피드</div>
            </div>
            <div className="list-meta">
              <span>
                {filteredNotices.length} / {notices.length}건
              </span>
              <span>선택 필터 기준으로 정렬됨</span>
            </div>
          </div>

          {sortedNotices.length === 0 ? (
            <div className="empty-state">
              <h3 className="notice-title">조건에 맞는 공지가 없습니다</h3>
              <p className="hero-subtitle">
                필터를 줄이거나 검색어를 변경해 다시 시도해보세요.
              </p>
              <button type="button" onClick={resetFilters}>
                필터 초기화
              </button>
            </div>
          ) : (
            <div className="notice-grid">
              {sortedNotices.map((notice, index) => (
                <a
                  key={`${notice.link}-${notice.department_id}-${notice.board_id}-${index}`}
                  href={notice.link}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="notice-card"
                  style={{ animationDelay: `${index * 40}ms` }}
                >
                  <div className="notice-kicker">
                    {notice.campus} / {notice.department_name}
                  </div>
                  <div className="notice-title line-clamp-2">
                    {notice.title}
                  </div>
                  <div className="notice-tags">
                    <span className="tag tag-accent">{notice.board_name}</span>
                    <span className="tag tag-teal">{notice.department_id}</span>
                    <span className="tag">{notice.board_id}</span>
                  </div>
                  <div className="notice-footer">
                    <span className="notice-date">{notice.date || '날짜 미상'}</span>
                    <span className="notice-link">자세히 보기</span>
                  </div>
                </a>
              ))}
            </div>
          )}
        </section>
      </main>

      <footer className="footer">
        <p>uRing Viewer - Yonsei University Notice Monitor</p>
        <p>캠퍼스 공지 데이터를 정제하여 하나의 피드로 제공합니다.</p>
      </footer>
    </div>
  );
}
