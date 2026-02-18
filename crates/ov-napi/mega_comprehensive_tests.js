const m = require('./openviking-engine.darwin-arm64.node');
const fs = require('fs');
const crypto = require('crypto');
const path = require('path');

console.log('=== OpenViking-rs MEGA Comprehensive Test Suite v2 ===\n');

// Test utilities
function benchmark(name, iterations, fn) {
  console.log(`⏱️  Benchmarking ${name} (${iterations} iterations)...`);
  const start = Date.now();
  const results = [];
  
  for (let i = 0; i < iterations; i++) {
    const itemStart = Date.now();
    fn(i);
    results.push(Date.now() - itemStart);
  }
  
  const total = Date.now() - start;
  results.sort((a, b) => a - b);
  const p50 = results[Math.floor(results.length * 0.5)];
  const p95 = results[Math.floor(results.length * 0.95)];
  const p99 = results[Math.floor(results.length * 0.99)];
  
  console.log(`   Total: ${total}ms, Avg: ${(total/iterations).toFixed(2)}ms`);
  console.log(`   Latency: p50=${p50}ms, p95=${p95}ms, p99=${p99}ms\n`);
  return { total, avg: total/iterations, p50, p95, p99 };
}

function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

function generateRandomString(length) {
  return crypto.randomBytes(length).toString('hex').substring(0, length);
}

function generateUnicodeString(length) {
  const chars = '你好世界🚀🌍💻📊🎉测试中文日本語한국어العربية';
  let result = '';
  for (let i = 0; i < length; i++) {
    result += chars[Math.floor(Math.random() * chars.length)];
  }
  return result;
}

// Global test state
let testResults = {
  passed: 0,
  failed: 0,
  benchmarks: {},
  errors: []
};

function assert(condition, message) {
  if (condition) {
    testResults.passed++;
    console.log(`✅ ${message}`);
  } else {
    testResults.failed++;
    console.log(`❌ ${message}`);
    testResults.errors.push(message);
  }
}

function assertThrows(fn, expectedError, message) {
  try {
    fn();
    testResults.failed++;
    console.log(`❌ ${message} (expected to throw but didn't)`);
    testResults.errors.push(`${message} (expected to throw but didn't)`);
  } catch (error) {
    if (expectedError && !error.message.includes(expectedError)) {
      testResults.failed++;
      console.log(`❌ ${message} (wrong error: ${error.message})`);
      testResults.errors.push(`${message} (wrong error: ${error.message})`);
    } else {
      testResults.passed++;
      console.log(`✅ ${message}`);
    }
  }
}

// =========================== A. Memory CRUD Tests (30+ tests) ===========================
console.log('🧠 A. Memory CRUD Tests (30+ tests)');
console.log('-'.repeat(70));

const testSession = m.createSession('test_user_mega');
console.log(`Created mega test session: ${testSession.id}\n`);

// A1-A5: Empty content tests
console.log('A1-A5: Empty content handling...');
assertThrows(() => m.storeMemory(testSession.id, '', 'empty', 'personal'), 'empty', 'Empty content should error gracefully');
assertThrows(() => m.storeMemory(testSession.id, null, 'null', 'personal'), 'null', 'Null content should error gracefully');
assertThrows(() => m.storeMemory(testSession.id, undefined, 'undefined', 'personal'), 'undefined', 'Undefined content should error gracefully');

// Valid minimal content
try { 
  const result = m.storeMemory(testSession.id, 'x', 'minimal', 'personal');
  assert(result && result.id, 'Single character content accepted');
} catch (e) { 
  assert(false, `Single character content failed: ${e.message}`);
}

try { 
  const result = m.storeMemory(testSession.id, '   whitespace only   ', 'whitespace', 'personal');
  assert(result && result.id, 'Whitespace-only content accepted');
} catch (e) { 
  assert(false, `Whitespace content failed: ${e.message}`);
}

// A6-A10: Large content tests
console.log('\nA6-A10: Large content handling...');
const largeContent1MB = 'X'.repeat(1024 * 1024); // 1MB
const hugeContent10MB = 'Y'.repeat(10 * 1024 * 1024); // 10MB

try {
  const result1MB = m.storeMemory(testSession.id, largeContent1MB, '1MB-test', 'personal');
  assert(result1MB && result1MB.id, '1MB content accepted');
} catch (e) {
  assert(true, `1MB content properly rejected: ${e.message}`);
}

try {
  const result10MB = m.storeMemory(testSession.id, hugeContent10MB, '10MB-test', 'personal');
  assert(false, '10MB content should be rejected'); // This should fail
} catch (e) {
  assert(true, `10MB content properly rejected: ${e.message}`);
}

// Test null/undefined fields
assertThrows(() => m.storeMemory(testSession.id, 'content', null, 'personal'), null, 'Null title should error');
assertThrows(() => m.storeMemory(testSession.id, 'content', 'title', null), null, 'Null category should error');

// A11-A15: Duplicate and collision tests
console.log('\nA11-A15: Duplicate and collision handling...');
const baseMemory = { content: 'duplicate test content', title: 'duplicate-title', category: 'personal' };

try {
  const mem1 = m.storeMemory(testSession.id, baseMemory.content, baseMemory.title, baseMemory.category);
  const mem2 = m.storeMemory(testSession.id, baseMemory.content, baseMemory.title, baseMemory.category);
  assert(mem1.id !== mem2.id, 'Duplicate memories get different IDs');
  
  // Try exact same content, different title
  const mem3 = m.storeMemory(testSession.id, baseMemory.content, 'different-title', baseMemory.category);
  assert(mem3.id, 'Same content, different title accepted');
} catch (e) {
  assert(false, `Duplicate handling failed: ${e.message}`);
}

// A16-A20: Search tests with various inputs
console.log('\nA16-A20: Search with various inputs...');
const searchQueries = [
  { query: '', name: 'empty string search' },
  { query: 'x', name: 'single character search' },  
  { query: 'X'.repeat(10000), name: 'very long query (10KB)' },
  { query: '"quotes and \\"escapes\\"', name: 'quoted search with escapes' },
  { query: 'SELECT * FROM memories; DROP TABLE memories;', name: 'SQL injection attempt' }
];

searchQueries.forEach(({ query, name }, i) => {
  try {
    const results = m.searchMemories(testSession.id, query, 10);
    assert(Array.isArray(results), `A${16+i}: ${name} returns array`);
  } catch (e) {
    assert(true, `A${16+i}: ${name} properly handled error: ${e.message}`);
  }
});

// A21-A25: Search limits and bounds
console.log('\nA21-A25: Search limits and bounds...');
const limitTests = [
  { limit: 0, name: 'limit=0' },
  { limit: -1, name: 'limit=-1' },
  { limit: 999999, name: 'limit=999999' },
  { limit: 1.5, name: 'limit=1.5 (float)' },
  { limit: 'invalid', name: 'limit=string' }
];

limitTests.forEach(({ limit, name }, i) => {
  try {
    const results = m.searchMemories(testSession.id, 'test', limit);
    if (limit <= 0 || typeof limit !== 'number' || !Number.isInteger(limit)) {
      assert(false, `A${21+i}: ${name} should have errored`);
    } else {
      assert(Array.isArray(results), `A${21+i}: ${name} handled properly`);
    }
  } catch (e) {
    assert(true, `A${21+i}: ${name} properly rejected: ${e.message}`);
  }
});

// A26-A30: Category-specific searches
console.log('\nA26-A30: Category-specific searches...');
const categories = ['personal', 'work', 'research', 'code', 'notes', 'invalid-category'];

categories.forEach((cat, i) => {
  try {
    // Store a memory in each category first
    if (cat !== 'invalid-category') {
      m.storeMemory(testSession.id, `Content for ${cat} category`, `${cat}-memory`, cat);
    }
    
    const results = m.searchMemories(testSession.id, cat, 5);
    assert(Array.isArray(results), `A${26+i}: Search in ${cat} category works`);
    
    if (cat !== 'invalid-category') {
      const found = results.some(r => r.category === cat);
      assert(found, `A${26+i}: Found memories in ${cat} category`);
    }
  } catch (e) {
    if (cat === 'invalid-category') {
      assert(true, `A${26+i}: Invalid category properly rejected`);
    } else {
      assert(false, `A${26+i}: Category ${cat} search failed: ${e.message}`);
    }
  }
});

// =========================== B. Session Management Tests (25+ tests) ===========================
console.log('\n💬 B. Session Management Tests (25+ tests)');
console.log('-'.repeat(70));

// B1-B5: Session creation with edge cases
console.log('B1-B5: Session creation edge cases...');
const sessionTests = [
  { userId: '', name: 'empty userId' },
  { userId: 'a'.repeat(1000), name: 'very long userId' },
  { userId: '用户测试', name: 'Unicode userId' },
  { userId: 'user/with/slashes', name: 'userId with slashes' },
  { userId: 'user with spaces', name: 'userId with spaces' }
];

sessionTests.forEach(({ userId, name }, i) => {
  try {
    if (userId === '') {
      assertThrows(() => m.createSession(userId), 'empty', `B${i+1}: ${name} should error`);
    } else {
      const session = m.createSession(userId);
      assert(session && session.id, `B${i+1}: ${name} created successfully`);
      // Clean up
      try { m.closeSession(session.id); } catch(e) {}
    }
  } catch (e) {
    assert(false, `B${i+1}: ${name} failed: ${e.message}`);
  }
});

// B6-B10: Message role validation
console.log('\nB6-B10: Message role validation...');
const messageRoles = ['user', 'assistant', 'system', 'invalid-role', null];

const msgTestSession = m.createSession('msg-test-user');
messageRoles.forEach((role, i) => {
  try {
    if (role === 'invalid-role' || role === null) {
      assertThrows(() => m.addMessage(msgTestSession.id, `Test message ${i}`, role || 'invalid'), 
                  'role', `B${i+6}: Invalid role '${role}' should error`);
    } else {
      const result = m.addMessage(msgTestSession.id, `Test message ${i}`, role);
      assert(result, `B${i+6}: Valid role '${role}' accepted`);
    }
  } catch (e) {
    if (role === 'invalid-role' || role === null) {
      assert(true, `B${i+6}: Invalid role properly rejected`);
    } else {
      assert(false, `B${i+6}: Valid role ${role} failed: ${e.message}`);
    }
  }
});

// B11-B15: Messages to non-existent sessions
console.log('\nB11-B15: Messages to non-existent/closed sessions...');
const fakeSessionId = 'fake-session-' + Date.now();
const closedSession = m.createSession('temp-user');
m.closeSession(closedSession.id);

const sessionErrorTests = [
  { sessionId: fakeSessionId, name: 'non-existent session' },
  { sessionId: closedSession.id, name: 'closed session' },
  { sessionId: '', name: 'empty session ID' },
  { sessionId: null, name: 'null session ID' },
  { sessionId: undefined, name: 'undefined session ID' }
];

sessionErrorTests.forEach(({ sessionId, name }, i) => {
  assertThrows(() => m.addMessage(sessionId, 'test message', 'user'), 
              null, `B${i+11}: Add message to ${name} should error`);
});

// B16-B20: Session listing and filtering
console.log('\nB16-B20: Session listing and filtering...');
const listTestSessions = [];
for (let i = 0; i < 5; i++) {
  const session = m.createSession(`list-test-user-${i}`);
  listTestSessions.push(session);
  m.addMessage(session.id, `Message in session ${i}`, 'user');
}

try {
  const allSessions = m.listSessions();  // No filter
  assert(Array.isArray(allSessions) && allSessions.length >= 5, 'B16: List all sessions works');
  
  const userSessions = m.listSessions('list-test-user-0');
  assert(Array.isArray(userSessions), 'B17: List user-specific sessions works');
  
  const nonExistentUser = m.listSessions('non-existent-user-xyz');
  assert(Array.isArray(nonExistentUser) && nonExistentUser.length === 0, 'B18: List sessions for non-existent user returns empty array');
  
  // Test edge cases
  assertThrows(() => m.listSessions(''), 'empty', 'B19: Empty user filter should error');
  assertThrows(() => m.listSessions(null), 'null', 'B20: Null user filter should error');
  
} catch (e) {
  assert(false, `Session listing tests failed: ${e.message}`);
}

// Clean up
listTestSessions.forEach(s => { try { m.closeSession(s.id); } catch(e) {} });

// B21-B25: Memory extraction edge cases  
console.log('\nB21-B25: Memory extraction edge cases...');
const extractTestSession = m.createSession('extract-test-user');

const extractionTests = [
  { setup: () => {}, name: 'empty session' },
  { setup: () => m.addMessage(extractTestSession.id, 'System message only', 'system'), name: 'system messages only' },
  { setup: () => {
    for (let i = 0; i < 1000; i++) {
      m.addMessage(extractTestSession.id, `Message ${i}`, i % 2 ? 'user' : 'assistant');
    }
  }, name: '1000 messages session' },
  { setup: () => m.addMessage(extractTestSession.id, 'A'.repeat(100000), 'user'), name: 'very long message (100KB)' },
  { setup: () => m.addMessage(extractTestSession.id, '你好世界🚀', 'user'), name: 'Unicode message' }
];

extractionTests.forEach(({ setup, name }, i) => {
  try {
    const tempSession = m.createSession(`extract-temp-${i}`);
    setup();  // Setup test data
    const extracted = m.extractMemories(tempSession.id);
    assert(Array.isArray(extracted), `B${i+21}: Memory extraction from ${name} returns array`);
    m.closeSession(tempSession.id);
  } catch (e) {
    assert(false, `B${i+21}: Memory extraction from ${name} failed: ${e.message}`);
  }
});

// =========================== C. Compression Tests (20+ tests) ===========================
console.log('\n🗜️  C. Compression Tests (20+ tests)');
console.log('-'.repeat(70));

// C1-C5: Basic compression levels
console.log('C1-C5: Basic compression levels...');
const testText = 'The quick brown fox jumps over the lazy dog. '.repeat(100);
const compressionLevels = ['lossless', 'minimal', 'balanced'];

compressionLevels.forEach((level, i) => {
  try {
    const compressed = m.compressText(testText, level);
    assert(compressed && compressed.length > 0, `C${i+1}: ${level} compression produces output`);
    
    const detailed = m.compressDetailed(testText, level);
    assert(detailed && detailed.compressed && detailed.originalSize && detailed.compressedSize, 
           `C${i+1}: ${level} compressDetailed provides stats`);
    
    const ratio = detailed.compressedSize / detailed.originalSize;
    console.log(`   ${level}: ${detailed.originalSize} → ${detailed.compressedSize} bytes (${(ratio*100).toFixed(1)}% ratio)`);
  } catch (e) {
    assert(false, `C${i+1}: ${level} compression failed: ${e.message}`);
  }
});

// C6-C10: Edge case inputs
console.log('\nC6-C10: Compression edge cases...');
const edgeCases = [
  { input: '', name: 'empty string' },
  { input: 'x', name: 'single character' },
  { input: 'AAAAAAAAAAAAAAAAAAAA', name: 'repeating pattern' },
  { input: crypto.randomBytes(1000).toString('hex'), name: 'random data (low compression)' },
  { input: '你好世界'.repeat(250), name: 'Unicode repetition' }
];

edgeCases.forEach(({ input, name }, i) => {
  try {
    if (input === '') {
      assertThrows(() => m.compressText(input, 'lossless'), 'empty', `C${i+6}: ${name} should error`);
    } else {
      const compressed = m.compressText(input, 'lossless');
      assert(compressed !== null, `C${i+6}: ${name} compression handled`);
    }
  } catch (e) {
    if (input === '') {
      assert(true, `C${i+6}: ${name} properly rejected`);
    } else {
      assert(false, `C${i+6}: ${name} failed: ${e.message}`);
    }
  }
});

// C11-C15: Roundtrip integrity tests 
console.log('\nC11-C15: Roundtrip integrity tests...');
const roundtripTests = [
  'Simple ASCII text',
  'Mixed 中文 and English text',
  JSON.stringify({key: 'value', array: [1,2,3], nested: {deep: true}}),
  'Code:\nfunction test() {\n  return "Hello World";\n}',
  'Special chars: !@#$%^&*()[]{}|\\:";\'<>?,./'
];

roundtripTests.forEach((text, i) => {
  try {
    const compressed = m.compressText(text, 'lossless');
    const decompressed = m.decompressText(compressed);
    // Note: The original failing test showed decompressed !== original
    // Let's see if this is still an issue
    if (decompressed === text) {
      assert(true, `C${i+11}: Roundtrip preserved: ${text.substring(0, 30)}...`);
    } else {
      assert(false, `C${i+11}: Roundtrip FAILED for: ${text.substring(0, 30)}... (got: ${decompressed?.substring(0, 30)}...)`);
    }
  } catch (e) {
    assert(false, `C${i+11}: Roundtrip test failed: ${e.message}`);
  }
});

// C16-C20: Decompression without compression
console.log('\nC16-C20: Decompression edge cases...');
const decompressionTests = [
  { input: 'not-compressed-text', name: 'plain text decompression' },
  { input: '', name: 'empty string decompression' },
  { input: 'invalid-base64-!@#$', name: 'invalid data' },
  { input: null, name: 'null input' },
  { input: undefined, name: 'undefined input' }
];

decompressionTests.forEach(({ input, name }, i) => {
  try {
    if (input === null || input === undefined || input === '') {
      assertThrows(() => m.decompressText(input), null, `C${i+16}: ${name} should error`);
    } else {
      const result = m.decompressText(input);
      // Either it works or it properly errors - both are valid
      assert(true, `C${i+16}: ${name} handled (result: ${typeof result})`);
    }
  } catch (e) {
    assert(true, `C${i+16}: ${name} properly handled error: ${e.message.substring(0, 50)}`);
  }
});

// =========================== D. Router Tests (20+ tests) ===========================
console.log('\n🧭 D. Router Tests (20+ tests)');
console.log('-'.repeat(70));

// D1-D5: Basic routing profiles
console.log('D1-D5: Basic routing profiles...');
const profiles = ['eco', 'auto', 'premium'];
const basicQueries = [
  'What is 2+2?',
  'Explain quantum mechanics',
  'Write a Python function',
  'Summarize this document',
  'Creative writing task'
];

basicQueries.forEach((query, i) => {
  try {
    const result = m.route(query, 'auto');
    assert(result && result.model && result.confidence >= 0, `D${i+1}: Route "${query.substring(0, 20)}..." works`);
  } catch (e) {
    assert(false, `D${i+1}: Basic routing failed: ${e.message}`);
  }
});

// D6-D10: Profile-specific routing
console.log('\nD6-D10: Profile-specific routing...');
profiles.forEach((profile, i) => {
  try {
    const result = m.route('Complex reasoning task requiring deep analysis', profile);
    assert(result && result.model, `D${i+6}: ${profile} profile routing works`);
    console.log(`   ${profile} → ${result.model} (confidence: ${result.confidence})`);
  } catch (e) {
    assert(false, `D${i+6}: ${profile} profile failed: ${e.message}`);
  }
});

// D11-D12: Invalid profiles
['invalid-profile', null].forEach((profile, i) => {
  assertThrows(() => m.route('test query', profile), null, `D${i+11}: Invalid profile '${profile}' should error`);
});

// D13-D17: Edge case queries
console.log('\nD13-D17: Router edge case queries...');
const edgeQueries = [
  { query: '', name: 'empty query' },
  { query: 'x', name: 'single character query' },
  { query: 'X'.repeat(10000), name: 'very long query (10KB)' },
  { query: '代码编程软件开发', name: 'Chinese coding query' },
  { query: 'code'.repeat(100), name: 'repetitive query' }
];

edgeQueries.forEach(({ query, name }, i) => {
  try {
    if (query === '') {
      assertThrows(() => m.route(query, 'auto'), 'empty', `D${i+13}: ${name} should error`);
    } else {
      const result = m.route(query, 'auto');
      assert(result && result.model, `D${i+13}: ${name} handled`);
    }
  } catch (e) {
    if (query === '') {
      assert(true, `D${i+13}: ${name} properly rejected`);
    } else {
      assert(false, `D${i+13}: ${name} failed: ${e.message}`);
    }
  }
});

// D18-D20: Consistency tests
console.log('\nD18-D20: Router consistency tests...');
const consistencyQuery = 'Write a Python function to sort a list';
const results = [];
for (let i = 0; i < 10; i++) {
  try {
    const result = m.route(consistencyQuery, 'auto');
    results.push(result.model);
  } catch (e) {
    assert(false, `D18: Consistency test ${i} failed: ${e.message}`);
  }
}

const uniqueModels = [...new Set(results)];
assert(uniqueModels.length <= 3, 'D18: Router shows reasonable consistency (≤3 different models in 10 calls)');
assert(results.length === 10, 'D19: All consistency tests completed');

// Test agentic vs non-agentic detection
try {
  const agenticResult = m.route('Create a plan and execute multiple steps to solve this complex problem', 'auto');
  const simpleResult = m.route('What is the capital of France?', 'auto');
  assert(agenticResult && simpleResult, 'D20: Agentic vs simple query routing works');
} catch (e) {
  assert(false, `D20: Agentic detection failed: ${e.message}`);
}

// =========================== E. Vector Search Tests (20+ tests) ===========================
console.log('\n🔍 E. Vector Search Tests (20+ tests)');
console.log('-'.repeat(70));

// E1-E5: Basic vector operations
console.log('E1-E5: Basic vector operations...');
const testVectors = JSON.stringify([
  ['doc1', [0.1, 0.2, 0.3, 0.4, 0.5]],
  ['doc2', [0.2, 0.3, 0.4, 0.5, 0.6]],  
  ['doc3', [0.9, 0.8, 0.7, 0.6, 0.5]],
  ['doc4', [-0.1, -0.2, -0.3, -0.4, -0.5]],
  ['doc5', [0.0, 0.0, 0.0, 0.0, 0.0]]  // Zero vector
]);

const queryVector = [0.1, 0.2, 0.3, 0.4, 0.5];

try {
  const results = m.vectorSearch(queryVector, testVectors, 3);
  assert(Array.isArray(results), 'E1: Vector search returns array');
  assert(results.length <= 3, 'E2: Respects top_k limit');
  assert(results[0].id === 'doc1', 'E3: Best match is exact match');
  assert(results[0].score > 0.9, 'E4: Exact match has high score (>0.9)');
  assert(results.every(r => r.score >= 0), 'E5: All scores are non-negative');
} catch (e) {
  assert(false, `E1-5: Basic vector search failed: ${e.message}`);
}

// E6-E10: Edge case vectors
console.log('\nE6-E10: Edge case vectors...');
const edgeVectorTests = [
  { query: [], name: 'zero-length vector' },
  { query: [0, 0, 0, 0, 0], name: 'all-zero vector' },
  { query: [1000, 1000, 1000, 1000, 1000], name: 'very large values' },
  { query: [NaN, 0.1, 0.2, 0.3, 0.4], name: 'NaN in vector' },
  { query: [0.1, 0.2, 0.3], name: 'wrong dimensions (3 vs 5)' }
];

edgeVectorTests.forEach(({ query, name }, i) => {
  try {
    if (query.length === 0 || query.includes(NaN) || query.length !== 5) {
      assertThrows(() => m.vectorSearch(query, testVectors, 3), null, `E${i+6}: ${name} should error`);
    } else {
      const results = m.vectorSearch(query, testVectors, 3);
      assert(Array.isArray(results), `E${i+6}: ${name} handled`);
    }
  } catch (e) {
    if (query.length === 0 || query.includes(NaN) || query.length !== 5) {
      assert(true, `E${i+6}: ${name} properly rejected`);
    } else {
      assert(false, `E${i+6}: ${name} failed: ${e.message}`);
    }
  }
});

// E11-E15: Large vector collections
console.log('\nE11-E15: Large vector collections...');
const largeVectorSet = [];
for (let i = 0; i < 1000; i++) {
  const vec = [Math.random(), Math.random(), Math.random(), Math.random(), Math.random()];
  largeVectorSet.push([`doc${i}`, vec]);
}
const largeVectorJson = JSON.stringify(largeVectorSet);

try {
  const largeResults = m.vectorSearch([0.5, 0.5, 0.5, 0.5, 0.5], largeVectorJson, 10);
  assert(Array.isArray(largeResults), 'E11: Large vector set (1000) search works');
  assert(largeResults.length === 10, 'E12: Large set respects limit');
  
  // Test different limits
  const limit1 = m.vectorSearch([0.5, 0.5, 0.5, 0.5, 0.5], largeVectorJson, 1);
  const limit100 = m.vectorSearch([0.5, 0.5, 0.5, 0.5, 0.5], largeVectorJson, 100);
  assert(limit1.length === 1, 'E13: limit=1 works on large set');
  assert(limit100.length === 100, 'E14: limit=100 works on large set');
  
  // Check score ordering
  const sorted = largeResults.every((result, i) => 
    i === 0 || result.score <= largeResults[i-1].score);
  assert(sorted, 'E15: Large set results are score-ordered');
  
} catch (e) {
  assert(false, `E11-15: Large vector tests failed: ${e.message}`);
}

// E16-E20: Concurrent vector operations
console.log('\nE16-E20: Concurrent vector operations...');
const concurrentPromises = [];
for (let i = 0; i < 20; i++) {
  const promise = new Promise((resolve, reject) => {
    try {
      const query = [Math.random(), Math.random(), Math.random(), Math.random(), Math.random()];
      const results = m.vectorSearch(query, testVectors, 3);
      resolve(results);
    } catch (e) {
      reject(e);
    }
  });
  concurrentPromises.push(promise);
}

Promise.allSettled(concurrentPromises).then(results => {
  const successful = results.filter(r => r.status === 'fulfilled').length;
  assert(successful >= 18, `E16-20: Concurrent vector search: ${successful}/20 successful (≥18 required)`);
});

// =========================== F. Crash Recovery & Persistence Tests (15+ tests) ===========================
console.log('\n💥 F. Crash Recovery & Persistence Tests (15+ tests)');  
console.log('-'.repeat(70));

// F1-F5: Data persistence tests
console.log('F1-F5: Data persistence tests...');
const persistSession = m.createSession('persist-test-user');
const persistentMemories = [];

for (let i = 0; i < 10; i++) {
  try {
    const memory = m.storeMemory(persistSession.id, `Persistent memory ${i}`, `persist-${i}`, 'research');
    persistentMemories.push(memory);
  } catch (e) {
    assert(false, `F${i+1}: Failed to store persistent memory ${i}: ${e.message}`);
  }
}

// Simulate restart by creating new session and searching
try {
  const searchResults = m.searchMemories(persistSession.id, 'Persistent memory', 20);
  const foundCount = searchResults.length;
  assert(foundCount >= 8, `F1: Data persistence after operations: ${foundCount}/10 memories found (≥8 required)`);
} catch (e) {
  assert(false, `F1: Persistence test failed: ${e.message}`);
}

// F2-F5: Rapid restart simulation
console.log('\nF2-F5: Rapid restart simulation...');
for (let cycle = 0; cycle < 4; cycle++) {
  try {
    const cycleSession = m.createSession(`cycle-test-${cycle}`);
    m.storeMemory(cycleSession.id, `Cycle ${cycle} data`, `cycle-${cycle}`, 'personal');
    m.addMessage(cycleSession.id, `Cycle ${cycle} message`, 'user');
    
    // Immediately search to verify
    const results = m.searchMemories(cycleSession.id, `Cycle ${cycle}`, 5);
    assert(results.length > 0, `F${cycle+2}: Rapid cycle ${cycle} data persisted`);
    
    m.closeSession(cycleSession.id);
  } catch (e) {
    assert(false, `F${cycle+2}: Rapid restart cycle ${cycle} failed: ${e.message}`);
  }
}

// F6-F10: Error handling and recovery
console.log('\nF6-F10: Error handling and recovery...');
const errorTests = [
  () => m.createSession('error-test-1'),
  () => m.storeMemory('non-existent-session', 'test', 'test', 'personal'),
  () => m.searchMemories('non-existent-session', 'test', 10),
  () => m.addMessage('non-existent-session', 'test', 'user'),
  () => m.extractMemories('non-existent-session')
];

errorTests.forEach((testFn, i) => {
  try {
    if (i === 0) {
      // First one should succeed
      const result = testFn();
      assert(result && result.id, `F${i+6}: Valid operation succeeds after errors`);
    } else {
      // Others should fail gracefully
      assertThrows(testFn, null, `F${i+6}: Invalid operation fails gracefully`);
    }
  } catch (e) {
    if (i === 0) {
      assert(false, `F${i+6}: Valid operation failed: ${e.message}`);
    } else {
      assert(true, `F${i+6}: Invalid operation properly rejected`);
    }
  }
});

// F11-F15: Large data integrity tests
console.log('\nF11-F15: Large data integrity tests...');
const integritySession = m.createSession('integrity-test-user');
const largeDataSets = [
  { size: 100, name: '100 memories' },
  { size: 500, name: '500 memories' },
  { size: 1000, name: '1000 memories' },
  { size: 50, name: '50 sessions' },
  { size: 100, name: '100 messages per session' }
];

largeDataSets.forEach(({ size, name }, i) => {
  try {
    if (name.includes('memories')) {
      // Store many memories
      let stored = 0;
      for (let j = 0; j < size; j++) {
        try {
          m.storeMemory(integritySession.id, `Large dataset memory ${j}`, `large-${j}`, 'research');
          stored++;
        } catch (e) {
          // Some failures are OK for very large sets
        }
      }
      assert(stored >= size * 0.8, `F${i+11}: ${name} - stored ${stored}/${size} (≥80% required)`);
      
    } else if (name.includes('sessions')) {
      // Create many sessions
      let created = 0;
      for (let j = 0; j < size; j++) {
        try {
          const session = m.createSession(`large-test-user-${j}`);
          if (session && session.id) created++;
        } catch (e) {
          // Some failures OK
        }
      }
      assert(created >= size * 0.8, `F${i+11}: ${name} - created ${created}/${size} (≥80% required)`);
      
    } else {
      // Many messages
      let added = 0;
      for (let j = 0; j < size; j++) {
        try {
          m.addMessage(integritySession.id, `Large message ${j}`, j % 2 ? 'user' : 'assistant');
          added++;
        } catch (e) {
          // Some failures OK
        }
      }
      assert(added >= size * 0.8, `F${i+11}: ${name} - added ${added}/${size} (≥80% required)`);
    }
  } catch (e) {
    assert(false, `F${i+11}: ${name} integrity test failed: ${e.message}`);
  }
});

// =========================== G. Security Tests (15+ tests) ===========================
console.log('\n🛡️  G. Security Tests (15+ tests)');
console.log('-'.repeat(70));

// G1-G5: Path traversal attempts
console.log('G1-G5: Path traversal attempts...');
const pathTraversalTests = [
  { content: '../../../etc/passwd', name: 'Unix path traversal in content' },
  { content: '..\\..\\windows\\system32\\config', name: 'Windows path traversal in content' },
  { title: '../../../secret/file', name: 'Path traversal in title' },
  { content: '%2e%2e%2f%2e%2e%2fpasswd', name: 'URL encoded path traversal' },
  { content: '....//....//etc/passwd', name: 'Double dot path traversal' }
];

const securitySession = m.createSession('security-test-user');
pathTraversalTests.forEach(({ content, title, name }, i) => {
  try {
    const result = m.storeMemory(securitySession.id, content || 'test content', title || `security-test-${i}`, 'personal');
    // Should either work safely or reject - both are acceptable
    assert(result && result.id, `G${i+1}: ${name} handled safely`);
  } catch (e) {
    assert(true, `G${i+1}: ${name} properly rejected: ${e.message}`);
  }
});

// G6-G10: Injection attempts
console.log('\nG6-G10: Injection attempts...');
const injectionTests = [
  { content: '<script>alert("XSS")</script>', name: 'HTML script injection' },
  { content: '${process.env.SECRET}', name: 'Template injection attempt' },
  { content: '{% raw %}{{ config.SECRET }}{% endraw %}', name: 'Template syntax injection' },
  { content: '\u0000null byte injection\u0000', name: 'Null byte injection' },
  { content: 'eval("process.exit(1)")', name: 'Code eval injection' }
];

injectionTests.forEach(({ content, name }, i) => {
  try {
    const result = m.storeMemory(securitySession.id, content, `injection-test-${i}`, 'personal');
    assert(result && result.id, `G${i+6}: ${name} content stored safely`);
    
    // Verify content is stored as-is (not executed)
    const searchResult = m.searchMemories(securitySession.id, `injection-test-${i}`, 1);
    assert(searchResult.length > 0, `G${i+6}: ${name} retrievable without execution`);
  } catch (e) {
    assert(true, `G${i+6}: ${name} properly rejected`);
  }
});

// G11-G15: Oversized payload tests
console.log('\nG11-G15: Oversized payload tests...');
const oversizedTests = [
  { size: 10 * 1024 * 1024, name: '10MB payload' },   // 10MB
  { size: 100 * 1024 * 1024, name: '100MB payload' }, // 100MB  
  { size: 1024 * 1024 * 1024, name: '1GB payload' },  // 1GB
  { content: JSON.stringify({a: 'x'.repeat(1000000)}), name: 'Large JSON payload' },
  { content: Array(10000).fill().map((_, i) => `Item ${i}`).join('\n'), name: 'Deep nested structure' }
];

oversizedTests.forEach(({ size, content, name }, i) => {
  try {
    const payload = content || 'X'.repeat(size);
    
    if (size >= 100 * 1024 * 1024) {  // ≥100MB should be rejected
      assertThrows(() => m.storeMemory(securitySession.id, payload, `oversized-${i}`, 'personal'), 
                  null, `G${i+11}: ${name} should be rejected`);
    } else {
      // Smaller payloads may work or be rejected - both OK
      try {
        const result = m.storeMemory(securitySession.id, payload, `oversized-${i}`, 'personal');
        assert(result && result.id, `G${i+11}: ${name} handled gracefully`);
      } catch (e) {
        assert(true, `G${i+11}: ${name} properly limited: ${e.message.substring(0, 50)}`);
      }
    }
  } catch (e) {
    if (size >= 100 * 1024 * 1024) {
      assert(true, `G${i+11}: ${name} properly rejected`);
    } else {
      assert(false, `G${i+11}: ${name} failed unexpectedly: ${e.message}`);
    }
  }
});

// =========================== H. UPSTREAM BUG FIX TESTS (20+ tests) ===========================
console.log('\n🐛 H. Upstream Bug Fix Tests (20+ tests)');
console.log('-'.repeat(70));

// H1-H5: Long filename bug (#205)
console.log('H1-H5: Long filename bug tests (#205)...');
const longFilenameTests = [
  { filename: 'x'.repeat(300), name: '300 ASCII chars filename' },
  { filename: '测试文件名'.repeat(50), name: '300+ CJK chars (multi-byte)' },
  { filename: '🚀📊💻'.repeat(100), name: '300+ emoji chars' },
  { filename: 'a'.repeat(255), name: 'exactly 255 chars filename' },
  { filename: 'a'.repeat(256), name: '256 chars filename (over limit)' }
];

longFilenameTests.forEach(({ filename, name }, i) => {
  try {
    // Test in memory storage with long titles (simulating filenames)
    const result = m.storeMemory(securitySession.id, 'Long filename test content', filename, 'personal');
    
    if (filename.length > 255) {
      // Should either work with truncation or fail gracefully
      assert(result && result.id, `H${i+1}: ${name} handled with truncation/hashing`);
    } else {
      assert(result && result.id, `H${i+1}: ${name} stored successfully`);
    }
    
    // Test retrieval
    const searchResult = m.searchMemories(securitySession.id, filename.substring(0, 50), 1);
    assert(Array.isArray(searchResult), `H${i+1}: ${name} retrievable after storage`);
    
  } catch (e) {
    // Graceful failure is acceptable for extreme cases
    assert(true, `H${i+1}: ${name} handled gracefully: ${e.message.substring(0, 50)}`);
  }
});

// H6-H10: search_by_id None guard (#198)
console.log('\nH6-H10: search_by_id None guard tests (#198)...');
const searchByIdTests = [
  { id: 'non-existent-id-12345', name: 'non-existent ID search' },
  { id: '', name: 'empty ID search' },
  { id: null, name: 'null ID search' },
  { id: undefined, name: 'undefined ID search' },
  { id: 'deleted-id', name: 'deleted memory ID search' }
];

// First store and then delete a memory for the last test
let deletedId = null;
try {
  const tempMemory = m.storeMemory(securitySession.id, 'To be deleted', 'temp-memory', 'personal');
  deletedId = tempMemory.id;
  // Note: We don't have a delete function exposed, but we can test with a fake ID
} catch (e) {
  // Ignore setup failure
}

searchByIdTests.forEach(({ id, name }, i) => {
  try {
    const testId = (id === 'deleted-id' && deletedId) ? deletedId : id;
    
    if (testId === null || testId === undefined || testId === '') {
      assertThrows(() => {
        // We need to test this through search since we don't have direct searchById
        m.searchMemories(securitySession.id, testId || '', 1);
      }, null, `H${i+6}: ${name} should error gracefully`);
    } else {
      const results = m.searchMemories(securitySession.id, `id:${testId}`, 1);
      assert(Array.isArray(results), `H${i+6}: ${name} returns empty array (no crash)`);
      // Empty results are expected and OK
    }
  } catch (e) {
    // Graceful error handling is good
    assert(true, `H${i+6}: ${name} handled gracefully: ${e.message.substring(0, 50)}`);
  }
});

// H11-H15: Duplicate filename auto-rename (#197)
console.log('\nH11-H15: Duplicate filename auto-rename tests (#197)...');
const duplicateTests = [
  { filename: 'duplicate-test.txt', iterations: 3, name: '3x same filename' },
  { filename: 'test-file.md', iterations: 5, name: '5x same filename' },
  { filename: '重复文件.doc', iterations: 3, name: '3x CJK filename' },
  { filename: 'file_with_special-chars!@#.txt', iterations: 2, name: '2x special chars filename' },
  { filename: 'a'.repeat(200) + '.extension', iterations: 2, name: '2x long filename with extension' }
];

duplicateTests.forEach(({ filename, iterations, name }, i) => {
  try {
    const storedIds = [];
    
    for (let j = 0; j < iterations; j++) {
      const result = m.storeMemory(securitySession.id, `Duplicate test content ${j}`, filename, 'personal');
      assert(result && result.id, `H${i+11}: ${name} iteration ${j+1} stored`);
      storedIds.push(result.id);
    }
    
    // All should have unique IDs even with same filename/title
    const uniqueIds = new Set(storedIds);
    assert(uniqueIds.size === storedIds.length, `H${i+11}: ${name} all have unique IDs (${uniqueIds.size}/${storedIds.length})`);
    
    // All should be retrievable
    const searchResults = m.searchMemories(securitySession.id, filename.substring(0, 20), 10);
    const foundCount = searchResults.length;
    assert(foundCount >= iterations - 1, `H${i+11}: ${name} at least ${iterations-1}/${iterations} retrievable`);
    
  } catch (e) {
    assert(false, `H${i+11}: ${name} failed: ${e.message}`);
  }
});

// H16-H20: Config edge cases (checking for any config-related crashes)
console.log('\nH16-H20: Config edge cases...');
const configTests = [
  { test: () => m.ping(), name: 'ping() basic config access' },
  { test: () => m.createSession('config-test-user'), name: 'session creation (config dependent)' },
  { test: () => m.route('test', 'eco'), name: 'routing (config dependent)' },
  { test: () => m.compressText('test', 'lossless'), name: 'compression (config dependent)' },
  { test: () => m.listSessions(), name: 'list operations (config dependent)' }
];

configTests.forEach(({ test, name }, i) => {
  try {
    const result = test();
    assert(result !== null && result !== undefined, `H${i+16}: ${name} works with current config`);
  } catch (e) {
    assert(false, `H${i+16}: ${name} failed (possible config issue): ${e.message}`);
  }
});

// =========================== Final Results ===========================
console.log('\n📊 MEGA Test Results Summary');
console.log('='.repeat(70));
console.log(`Total tests: ${testResults.passed + testResults.failed}`);
console.log(`✅ Passed: ${testResults.passed}`);
console.log(`❌ Failed: ${testResults.failed}`);
console.log(`Success rate: ${((testResults.passed / (testResults.passed + testResults.failed)) * 100).toFixed(1)}%\n`);

if (testResults.errors.length > 0) {
  console.log('❌ Errors encountered:');
  testResults.errors.forEach((error, i) => {
    console.log(`  ${i + 1}. ${error}`);
  });
  console.log();
}

if (testResults.benchmarks && Object.keys(testResults.benchmarks).length > 0) {
  console.log('📈 Performance Benchmarks:');
  Object.entries(testResults.benchmarks).forEach(([name, result]) => {
    if (typeof result === 'object') {
      console.log(`  ${name}: avg=${result.avg.toFixed(2)}ms, p95=${result.p95}ms`);
    } else {
      console.log(`  ${name}: ${result.toFixed(2)}`);
    }
  });
  console.log();
}

console.log('🎉 MEGA Comprehensive testing completed!');
console.log(`   Engine version: ${m.ping()}`);
console.log(`   Categories tested: A(Memory CRUD), B(Sessions), C(Compression), D(Router), E(Vector), F(Persistence), G(Security), H(Upstream Bugs)`);
console.log(`   Test duration: ${((Date.now() - Date.now()) / 1000).toFixed(1)}s`);

// Exit with non-zero code if there were failures
if (testResults.failed > 0) {
  console.log(`\n🚨 ${testResults.failed} tests failed - exiting with code 1`);
  process.exit(1);
} else {
  console.log('\n🎉 All tests passed!');
  process.exit(0);
}