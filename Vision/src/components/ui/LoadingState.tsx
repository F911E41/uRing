// src/components/ui/LoadingState.tsx

import { Spinner } from './Spinner';

interface LoadingStateProps {
    title?: string;
    description?: string;
}

export function LoadingState({
    title = 'Loading Notices',
    description = 'Fetching the latest notices by campus.'
}: LoadingStateProps) {
    return (
        <div className="state-screen">
            <div className="state-card">
                <Spinner />
                <h2 className="panel-title">{title}</h2>
                <p className="hero-subtitle">{description}</p>
            </div>
        </div>
    );
}
