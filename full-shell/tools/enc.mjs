// 印迹完整版构建：AES-256-GCM 加密/解密（口令文件不入库）
import { readFileSync, writeFileSync } from 'node:fs';
import { createCipheriv, createDecipheriv, scryptSync, randomBytes } from 'node:crypto';
const [, , mode, inPath, outPath, keyPath] = process.argv;
if (!['encrypt', 'decrypt'].includes(mode) || !inPath || !outPath || !keyPath) {
  throw new Error('usage: node enc.mjs encrypt|decrypt <in> <out> <keyfile>');
}
const pass = readFileSync(keyPath, 'utf8').trim();
const MAGIC = Buffer.from('YJENC1');
if (mode === 'encrypt') {
  const salt = randomBytes(16), iv = randomBytes(12);
  const key = scryptSync(pass, salt, 32);
  const cipher = createCipheriv('aes-256-gcm', key, iv);
  const plain = readFileSync(inPath);
  const body = Buffer.concat([cipher.update(plain), cipher.final()]);
  writeFileSync(outPath, Buffer.concat([MAGIC, salt, iv, cipher.getAuthTag(), body]));
  console.log(`encrypted ${plain.length} bytes -> ${outPath}`);
} else {
  const buf = readFileSync(inPath);
  if (buf.length < 50 || !buf.subarray(0, 6).equals(MAGIC)) throw new Error('bad magic / truncated');
  const salt = buf.subarray(6, 22), iv = buf.subarray(22, 34);
  const tag = buf.subarray(34, 50), body = buf.subarray(50);
  const key = scryptSync(pass, salt, 32);
  const decipher = createDecipheriv('aes-256-gcm', key, iv);
  decipher.setAuthTag(tag);
  const plain = Buffer.concat([decipher.update(body), decipher.final()]);
  writeFileSync(outPath, plain);
  console.log(`decrypted ${plain.length} bytes -> ${outPath}`);
}
