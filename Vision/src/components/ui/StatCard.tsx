// components/ui/StatCard.tsx

interface StatCardProps {
    label: string;
    value: number | string;
}

export const StatCard = ({ label, value }: StatCardProps) => (
    <div className="stat-card">
        <div className="stat-label">{label}</div>
        <div className="stat-value">{value}</div>
    </div>
);
