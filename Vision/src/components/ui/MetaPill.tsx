// src/components/ui/MetaPill.tsx

interface MetaPillProps {
    children: React.ReactNode;
}

export function MetaPill({ children }: MetaPillProps) {
    return <div className="meta-pill">{children}</div>;
}
