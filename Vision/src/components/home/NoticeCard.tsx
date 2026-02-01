// src/components/home/NoticeCard.tsx

interface NoticeCardProps {
    notice: {
        link: string;
        campus: string;
        department_name: string;
        title: string;
        board_name: string;
        department_id: string;
        board_id: string;
        date: string | null;
    };
    index: number;
}

export function NoticeCard({ notice, index }: NoticeCardProps) {
    return (
        <a
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
                <span className="notice-date">{notice.date || 'Date Unknown'}</span>
                <span className="notice-link">View Details</span>
            </div>
        </a>
    );
}
