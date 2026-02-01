// components/ui/Tag.tsx

interface TagProps {
    label: string;
    variant?: 'default' | 'accent' | 'teal';
}

export const Tag = ({ label, variant = 'default' }: TagProps) => {
    const className = variant === 'default' ? 'tag' : `tag tag-${variant}`;
    return <span className={className}>{label}</span>;
};
