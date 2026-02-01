// src/components/ui/Chip.tsx

interface ChipProps {
    children: React.ReactNode;
    onRemove?: () => void;
}

export function Chip({ children, onRemove }: ChipProps) {
    return (
        <span className="chip">
            {children}
            {onRemove && (
                <button onClick={onRemove} aria-label="Remove filter">
                    x
                </button>
            )}
        </span>
    );
}
