const { withAuth, loadKeys } = await import('./lib.js');

export default async function handler(req, res) {
  if (!(await withAuth(req, res))) return;

  try {
    const keys = await loadKeys();
    const list = Object.entries(keys).map(([code, record]) => ({
      code,
      max_activations: record.max_activations || 1,
      activation_count: record.activation_count || 0,
      machine_id: record.machine_id || null,
      activated_at: record.activated_at || null,
      expires_at: record.expires_at || null,
    }));

    return res.status(200).json({ keys: list });
  } catch (error) {
    console.error('list error:', error);
    return res.status(500).json({ error: '服务器内部错误' });
  }
}
