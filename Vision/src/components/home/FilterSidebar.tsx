// src/components/home/FilterSidebar.tsx

import { Chip } from '@components/ui';

interface FilterSidebarProps {
    // Search
    searchQuery: string;
    onSearchChange: (query: string) => void;

    // Filters
    selectedCampus: string | null;
    selectedDept: string | null;
    selectedBoard: string | null;
    onCampusChange: (campus: string | null) => void;
    onDeptChange: (dept: string | null) => void;
    onBoardChange: (board: string | null) => void;

    // Options
    campuses: string[];
    departments: string[];
    boards: string[];

    // Reset
    onResetFilters: () => void;
}

export function FilterSidebar({
    searchQuery,
    onSearchChange,
    selectedCampus,
    selectedDept,
    selectedBoard,
    onCampusChange,
    onDeptChange,
    onBoardChange,
    campuses,
    departments,
    boards,
    onResetFilters,
}: FilterSidebarProps) {
    const hasActiveFilters = selectedCampus || selectedDept || selectedBoard || searchQuery;

    return (
        <aside className="side-panel">
            {/* Quick Search */}
            <div className="panel-card">
                <div className="panel-title">Quick Search</div>
                <div className="input-wrap">
                    <input
                        className="text-input"
                        type="text"
                        value={searchQuery}
                        onChange={(e) => onSearchChange(e.target.value)}
                        placeholder="Enter title or keywords"
                    />
                    {searchQuery && (
                        <button
                            type="button"
                            className="input-clear"
                            aria-label="Clear search query"
                            onClick={() => onSearchChange('')}
                        >
                            x
                        </button>
                    )}
                </div>
            </div>

            {/* Filter Selection */}
            <div className="panel-card">
                <div className="panel-title">Filter Selection</div>
                <div className="space-y-4">
                    <div>
                        <label className="notice-kicker" htmlFor="campus-select">
                            Campus
                        </label>
                        <select
                            id="campus-select"
                            className="select-input"
                            value={selectedCampus || ''}
                            onChange={(e) => onCampusChange(e.target.value || null)}
                        >
                            <option value="">All Campuses</option>
                            {campuses.map((campus) => (
                                <option key={campus} value={campus}>
                                    {campus}
                                </option>
                            ))}
                        </select>
                    </div>

                    <div>
                        <label className="notice-kicker" htmlFor="dept-select">
                            Department
                        </label>
                        <select
                            id="dept-select"
                            className="select-input"
                            value={selectedDept || ''}
                            onChange={(e) => onDeptChange(e.target.value || null)}
                            disabled={!selectedCampus}
                        >
                            <option value="">All Departments</option>
                            {departments.map((dept) => (
                                <option key={dept} value={dept}>
                                    {dept}
                                </option>
                            ))}
                        </select>
                    </div>

                    <div>
                        <label className="notice-kicker" htmlFor="board-select">
                            Board
                        </label>
                        <select
                            id="board-select"
                            className="select-input"
                            value={selectedBoard || ''}
                            onChange={(e) => onBoardChange(e.target.value || null)}
                            disabled={!selectedCampus}
                        >
                            <option value="">All Boards</option>
                            {boards.map((board) => (
                                <option key={board} value={board}>
                                    {board}
                                </option>
                            ))}
                        </select>
                    </div>
                </div>
            </div>

            {/* Active Filters */}
            <div className="panel-card">
                <div className="panel-title">Active Filters</div>
                <div className="chip-row">
                    {selectedCampus && (
                        <Chip onRemove={() => onCampusChange(null)}>
                            {selectedCampus}
                        </Chip>
                    )}
                    {selectedDept && (
                        <Chip onRemove={() => onDeptChange(null)}>
                            {selectedDept}
                        </Chip>
                    )}
                    {selectedBoard && (
                        <Chip onRemove={() => onBoardChange(null)}>
                            {selectedBoard}
                        </Chip>
                    )}
                    {searchQuery && (
                        <Chip onRemove={() => onSearchChange('')}>
                            &quot;{searchQuery}&quot;
                        </Chip>
                    )}
                </div>
                {hasActiveFilters && (
                    <button type="button" className="reset-link" onClick={onResetFilters}>
                        Reset All
                    </button>
                )}
            </div>
        </aside>
    );
}
