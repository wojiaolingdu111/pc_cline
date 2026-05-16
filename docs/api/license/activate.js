// Vercel API: 激活授权码并绑定机器
//
// 绑定流程:
//   1. 验证授权码是否存在且未过期
//   2. 检查激活次数是否超限
//   3. 绑定 machine_id → 其他设备不可用此码
//   4. 返回激活结果

const KV_KEY = 'license:keys';

export default async function handler(req, res) {
  if (req.method !== 'POST') {
    return res.status(405).json({ valid: false, message: 'Method not allowed' });
  }

  const { license_key, machine_id } = req.body;
  if (!license_key) {
    return res.status(400).json({ valid: false, message: '缺少授权码' });
  }
  if (!machine_id) {
    return res.status(400).json({ valid: false, message: '缺少机器标识' });
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

    // 已绑定其他机器
    if (record.machine_id && record.machine_id !== machine_id) {
      return res.status(200).json({ valid: false, message: '授权码已被其他设备绑定' });
    }

    // 首次激活：绑定机器
    if (!record.machine_id) {
      const maxActivations = record.max_activations || 1;
      if ((record.activation_count || 0) >= maxActivations) {
        return res.status(200).json({ valid: false, message: '授权码激活次数已用完' });
      }
      record.machine_id = machine_id;
      record.activation_count = (record.activation_count || 0) + 1;
      record.activated_at = Date.now();

      await saveLicenseKeys(keys);
    }

    return res.status(200).json({
      valid: true,
      message: '激活成功',
      expires_at: record.expires_at || null,
    });
  } catch (error) {
    console.error('activate error:', error);
    return res.status(500).json({ valid: false, message: '服务器内部错误' });
  }
}

async function loadLicenseKeys() {
  if (process.env.KV_REST_API_URL && process.env.KV_REST_API_TOKEN) {
    const { get } = await import('@vercel/kv');
    const data = await get(KV_KEY);
    if (data) return data;
  }

  if (process.env.LICENSE_KEYS) {
    try {
      return JSON.parse(process.env.LICENSE_KEYS);
    } catch { /* ignore */ }
  }
  return {};
}

async function saveLicenseKeys(keys) {
  if (process.env.KV_REST_API_URL && process.env.KV_REST_API_TOKEN) {
    const { set } = await import('@vercel/kv');
    await set(KV_KEY, keys);
  }
  // 环境变量模式不支持回写
}
