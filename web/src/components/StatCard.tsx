import { Card, Statistic } from 'antd';

interface StatCardProps {
  title: string;
  value: number | string;
  prefix?: React.ReactNode;
  suffix?: string;
}

export default function StatCard({ title, value, prefix, suffix }: StatCardProps) {
  return (
    <Card>
      <Statistic title={title} value={value} prefix={prefix} suffix={suffix} />
    </Card>
  );
}
