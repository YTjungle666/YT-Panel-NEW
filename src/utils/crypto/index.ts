import CryptoJS from 'crypto-js'
import axios from 'axios'

/**
 * 前端加密模块
 * 
 * 安全设计：密钥不硬编码，而是从后端动态获取
 * - 每天轮换一次密钥
 * - 密钥不会出现在源代码中
 * - 即使代码泄露，没有当天密钥也无法解密
 */

// 内存中存储密钥（页面刷新会丢失，需要重新获取）
let cachedKey: string | null = null
let keyFetchedAt: number = 0
const KEY_VALID_DURATION = 1000 * 60 * 60 * 23 // 23小时，留1小时缓冲

/**
 * 从服务器获取加密密钥
 * 
 * 为什么这么做？
 * 1. 密钥不存储在前端代码中，防止泄露
 * 2. 密钥每天轮换，增加安全性
 * 3. 支持无密钥模式（明文存储），方便自托管用户
 */
async function fetchCryptoKey(): Promise<string | null> {
  // 如果内存中有有效密钥，直接返回
  const now = Date.now()
  if (cachedKey && (now - keyFetchedAt) < KEY_VALID_DURATION) {
    return cachedKey
  }

  try {
    // 从后端获取密钥
    const response = await axios.get('/api/crypto-key', { timeout: 5000 })
    if (response.data?.code === 200 && response.data?.data) {
      cachedKey = response.data.data
      keyFetchedAt = now
      return cachedKey
    }
  } catch (error) {
    // 如果获取失败，可能是后端未部署新版本
    // 降级为明文存储模式（自托管场景下安全足够）
    console.warn('Failed to fetch crypto key, using plaintext mode:', error)
    return null
  }
  
  return null
}

/**
 * 加密数据
 * 
 * @param data 要加密的数据
 * @returns 加密后的字符串（或明文JSON如果无密钥）
 */
export async function enCrypto(data: any): Promise<string> {
  const str = JSON.stringify(data)
  
  const key = await fetchCryptoKey()
  
  // 如果没有密钥，使用明文模式（兼容旧版本和自托管场景）
  if (!key) {
    return str
  }
  
  return CryptoJS.AES.encrypt(str, key).toString()
}

/**
 * 解密数据
 * 
 * @param data 加密字符串或明文JSON
 * @returns 原始数据
 */
export async function deCrypto(data: string): Promise<any> {
  if (!data) return null
  
  // 先尝试作为明文JSON解析（兼容模式和旧数据）
  try {
    return JSON.parse(data)
  } catch {
    // 不是明文，尝试解密
  }
  
  const key = await fetchCryptoKey()
  
  // 如果没有密钥，无法解密
  if (!key) {
    console.warn('No crypto key available, cannot decrypt')
    return null
  }
  
  try {
    const bytes = CryptoJS.AES.decrypt(data, key)
    const str = bytes.toString(CryptoJS.enc.Utf8)
    if (str) {
      return JSON.parse(str)
    }
  } catch (error) {
    console.warn('Decryption failed, data may be corrupted or key changed:', error)
  }
  
  return null
}

/**
 * 清除内存中的密钥
 * 用户登出时调用，防止密钥泄露
 */
export function clearCryptoKey(): void {
  cachedKey = null
  keyFetchedAt = 0
}
