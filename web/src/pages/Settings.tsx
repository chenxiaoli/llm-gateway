import { Typography, Switch, Card } from 'antd';
import { useSettings, useUpdateSettings } from '../hooks/useSettings';

const { Title } = Typography;

export default function Settings() {
  const { data: settings, isLoading } = useSettings();
  const updateMutation = useUpdateSettings();

  return (
    <div>
      <Title level={4}>Settings</Title>
      <Card loading={isLoading}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', maxWidth: 400 }}>
          <span>Allow Registration</span>
          <Switch
            checked={settings?.allow_registration ?? false}
            onChange={(checked) => updateMutation.mutate({ allow_registration: checked })}
          />
        </div>
      </Card>
    </div>
  );
}
