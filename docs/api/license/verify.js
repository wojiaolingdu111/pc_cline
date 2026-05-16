// Vercel API: 验证授权码
// 支持 Vercel KV (需配置) 和 环境变量 两种存储方式
//
// 环境变量配置:
//   LICENSE_KEYS={"LICENSE-XXXX": {"max_activations":3,"expires_at":null}}
// 或使用 Vercel KV (自动检测)

const KV_KEY = 'license:keys';

export default async function handler(req, res) {
  if (req.method !== 'POST') {
    return res.status(405).json({ valid: false, message: 'Method not allowed' });
  }

  const { license_key, machine_id } = req.body;
  if (!license_key) {
    return res.status(400).json({ valid: false, message: '缺少授权码' });
  }

  try {
    const keys = await loadLicenseKeys();
    const record = keys[license_key];

    if (!record) {
      return res.status(200).json({ valid: false, message: '授权码无效' });
    }

    if (record.expires_at && Date.now() > record.expires_at) {
      return res.status(200).json({ valid: false, message: '授权码已过期' });
    }

    // Check machine binding
    if (record.machine_id && record.machine_id !== machine_id) {
      return res.status(200).json({ valid: false, message: '授权码已被其他设备绑定' });
    }

    return res.status(200).json({
      valid: true,
      message: '授权码有效',
      expires_at: record.expires_at || null,
    });
  } catch (error) {
    console.error('verify error:', error);
    return res.status(500).json({ valid: false, message: '服务器内部错误' });
  }
}

async function loadLicenseKeys() {
  // 优先使用 Vercel KV
  if (process.env.KV_REST_API_URL && process.env.KV_REST_API_TOKEN) {
    const { get } = await import('@vercel/kv');
    const data = await get(KV_KEY);
    if (data) return data;
  }

  // 回退到环境变量
  if (process.env.LICENSE_KEYS) {
    try {
      return JSON.parse(process.env.LICENSE_KEYS);
    } catch { /* ignore */ }
  }

  // 默认空
  return {};
}
