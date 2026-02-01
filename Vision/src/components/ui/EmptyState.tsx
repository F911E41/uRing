// src/components/ui/EmptyState.tsx

interface EmptyStateProps {
    title: string;
    description: string;
    onReset?: () => void;
    resetLabel?: string;
}

export function EmptyState({
    title,
    description,
    onReset,
    resetLabel = 'Reset Filters'
}: EmptyStateProps) {
    return (
        <div className="empty-state">
            <h3 className="notice-title">{title}</h3>
            <p className="hero-subtitle">{description}</p>
            {onReset && (
                <button type="button" onClick={onReset}>
                    {resetLabel}
                </button>
            )}
        </div>
    );
}
