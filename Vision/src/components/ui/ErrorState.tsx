// src/components/ui/ErrorState.tsx

interface ErrorStateProps {
    title?: string;
    message: string;
}

export function ErrorState({
    title = 'Unable to Load Data',
    message
}: ErrorStateProps) {
    return (
        <div className="state-screen">
            <div className="state-card">
                <h2 className="panel-title">{title}</h2>
                <p className="hero-subtitle">{message}</p>
            </div>
        </div>
    );
}
