import { useState, useEffect } from 'react'
import { apiHeaders } from '../../lib/api'
import { Card, Row, Input, Select, Toggle, SaveBar } from './fields'

export default function ChannelsTab() {
  const [config, setConfig] = useState(null)
  const [saving, setSaving] = useState(false)
  const [status, setStatus] = useState(null)

  useEffect(() => {
    fetch('/api/settings/channels', { headers: apiHeaders() })
      .then(r => r.json())
      .then(d => setConfig(d))
      .catch(() => setStatus({ type: 'error', text: 'Failed to load' }))
  }, [])

  if (!config) return <div className="text-dim text-sm py-8 text-center">Loading...</div>

  const tg = config.telegram || { enabled: false, gap_minutes: 360, stream_mode: 'final_only' }
  const setTg = (key, val) => setConfig(prev => ({
    ...prev,
    telegram: { ...prev.telegram, [key]: val },
  }))

  const save = async () => {
    setSaving(true); setStatus(null)
    try {
      const resp = await fetch('/api/settings/channels', { method: 'PUT', headers: apiHeaders(), body: JSON.stringify(config) })
      setStatus(resp.ok ? { type: 'ok', text: 'Saved' } : { type: 'error', text: 'Failed' })
    } catch (e) { setStatus({ type: 'error', text: e.message }) }
    setSaving(false)
  }

  return (
    <>
      <Card title="Telegram">
        <Toggle label="Enabled" checked={tg.enabled} onChange={v => setTg('enabled', v)} />
        <Row label="Session gap" sub="minutes of inactivity before new session">
          <Input type="number" value={tg.gap_minutes ?? 360} onChange={v => setTg('gap_minutes', v === '' ? null : Number(v))} placeholder="360" />
        </Row>
        <Row label="Stream mode" sub="how messages are sent to Telegram">
          <Select value={tg.stream_mode || 'final_only'} onChange={v => setTg('stream_mode', v)} options={[
            { value: 'final_only', label: 'Final only' },
            { value: 'all_messages', label: 'All messages' },
          ]} />
        </Row>
        <div className="text-dim text-xs mt-2 px-1">Bot token is configured via TELEGRAM_BOT_TOKEN in .env</div>
      </Card>

      <SaveBar onSave={save} saving={saving} status={status} />
    </>
  )
}
