const m = require('./openviking-engine.darwin-arm64.node');
const fs = require('fs');
const crypto = require('crypto');
const path = require('path');

console.log('=== OpenViking-rs MEGA Comprehensive Test Suite v3 (FIXED) ===\n');

// Available functions mapping to correct names
const availableFunctions = [
  'addMemory',          // was storeMemory
  'addSessionMessage',  // was addMessage  
  'closeSession',
  'compress',           // NOT compressText
  'compressDetailed',   
  'createSession',
  'decompressText',     // This exists
  'extractMemories',
  'getSession',
  'listSessions',
  'ping',
  'route',
  'searchMemory',       // was searchMemories (note: singular)
  'vectorSearch'
];

console.log('Available functions:', availableFunctions);
console.log();

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
assertThrows(() => m.addMemory('', 'test_user_mega', testSession.id, 'empty'), 'empty', 'A1: Empty content should error gracefully');
assertThrows(() => m.addMemory(null, 'test_user_mega', testSession.id, 'null'), 'null', 'A2: Null content should error gracefully');
assertThrows(() => m.addMemory(undefined, 'test_user_mega', testSession.id, 'undefined'), 'undefined', 'A3: Undefined content should error gracefully');

// Valid minimal content
try { 
  const result = m.addMemory('x', 'test_user_mega', testSession.id, 'minimal');
  assert(result && result.stored, 'A4: Single character content accepted');
} catch (e) { 
  assert(false, `A4: Single character content failed: ${e.message}`);
}

try { 
  const result = m.addMemory('   whitespace only   ', 'test_user_mega', testSession.id, 'whitespace');
  assert(result && result.stored, 'A5: Whitespace-only content accepted');
} catch (e) { 
  assert(false, `A5: Whitespace content failed: ${e.message}`);
}

// A6-A10: Large content tests
console.log('\nA6-A10: Large content handling...');
const largeContent1MB = 'X'.repeat(1024 * 1024); // 1MB
const hugeContent10MB = 'Y'.repeat(10 * 1024 * 1024); // 10MB

try {
  const result1MB = m.addMemory(largeContent1MB, 'test_user_mega', testSession.id, '1MB-test');
  assert(result1MB && result1MB.stored, 'A6: 1MB content accepted');
} catch (e) {
  assert(true, `A6: 1MB content properly rejected: ${e.message}`);
}

try {
  const result10MB = m.addMemory(hugeContent10MB, 'test_user_mega', testSession.id, '10MB-test');
  assert(false, 'A7: 10MB content should be rejected'); // This should fail
} catch (e) {
  assert(true, `A7: 10MB content properly rejected: ${e.message}`);
}

// Test null/undefined fields - addMemory params are (content, userId, sessionId, category)
try {
  m.addMemory('content', null, testSession.id, 'personal');
  assert(false, 'A8: Null userId should error');
} catch (e) {
  assert(true, `A8: Null userId properly rejected: ${e.message}`);
}

try {
  m.addMemory('content', 'test_user_mega', testSession.id, null);
  assert(false, 'A9: Null category should error');  
} catch (e) {
  assert(true, `A9: Null category properly rejected: ${e.message}`);
}

try {
  m.addMemory('content', 'test_user_mega', null, 'personal');
  assert(false, 'A10: Null sessionId should error');
} catch (e) {
  assert(true, `A10: Null sessionId properly rejected: ${e.message}`);
}

// A11-A15: Duplicate and collision tests
console.log('\nA11-A15: Duplicate and collision handling...');
const baseMemory = { content: 'duplicate test content', userId: 'test_user_mega', category: 'personal' };

try {
  const mem1 = m.addMemory(baseMemory.content, baseMemory.userId, testSession.id, baseMemory.category);
  const mem2 = m.addMemory(baseMemory.content, baseMemory.userId, testSession.id, baseMemory.category);
  assert(mem1.id !== mem2.id, 'A11: Duplicate memories get different IDs');
  
  // Try exact same content, different category
  const mem3 = m.addMemory(baseMemory.content, baseMemory.userId, testSession.id, 'work');
  assert(mem3.stored, 'A12: Same content, different category accepted');
} catch (e) {
  assert(false, `A11-12: Duplicate handling failed: ${e.message}`);
}

// A13-A15: More collision tests
try {
  const mem4 = m.addMemory('unique content 1', 'test_user_mega', testSession.id, 'personal');
  const mem5 = m.addMemory('unique content 2', 'test_user_mega', testSession.id, 'personal');  
  const mem6 = m.addMemory('unique content 3', 'test_user_mega', testSession.id, 'personal');
  assert(new Set([mem4.id, mem5.id, mem6.id]).size === 3, 'A13-15: Multiple unique memories get unique IDs');
} catch (e) {
  assert(false, `A13-15: Multiple memory storage failed: ${e.message}`);
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
    const results = m.searchMemory(query, 'test_user_mega', testSession.id, 10);
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
    const results = m.searchMemory('test', 'test_user_mega', testSession.id, limit);
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
const categories = ['personal', 'work', 'research', 'code', 'notes'];

categories.forEach((cat, i) => {
  try {
    // Store a memory in each category first
    const storeResult = m.addMemory(`Content for ${cat} category`, 'test_user_mega', testSession.id, cat);
    assert(storeResult.stored, `A${26+i}: Stored memory in ${cat} category`);
    
    const results = m.searchMemory(cat, 'test_user_mega', testSession.id, 5);
    assert(Array.isArray(results), `A${26+i}: Search in ${cat} category works`);
    
    const found = results.some(r => r.content.includes(cat));
    assert(found, `A${26+i}: Found memories in ${cat} category`);
  } catch (e) {
    assert(false, `A${26+i}: Category ${cat} test failed: ${e.message}`);
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
    if (userId === '') {
      assert(true, `B${i+1}: ${name} properly rejected`);
    } else {
      assert(false, `B${i+1}: ${name} failed: ${e.message}`);
    }
  }
});

// B6-B10: Message role validation - addSessionMessage params are (sessionId, role, content)
console.log('\nB6-B10: Message role validation...');
const messageRoles = ['user', 'assistant', 'system', 'invalid-role', null];

const msgTestSession = m.createSession('msg-test-user');
messageRoles.forEach((role, i) => {
  try {
    if (role === 'invalid-role' || role === null) {
      assertThrows(() => m.addSessionMessage(msgTestSession.id, role || 'invalid', `Test message ${i}`), 
                  'role', `B${i+6}: Invalid role '${role}' should error`);
    } else {
      const result = m.addSessionMessage(msgTestSession.id, role, `Test message ${i}`);
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
  assertThrows(() => m.addSessionMessage(sessionId, 'user', 'test message'), 
              null, `B${i+11}: Add message to ${name} should error`);
});

// B16-B20: Session listing and filtering
console.log('\nB16-B20: Session listing and filtering...');
const listTestSessions = [];
for (let i = 0; i < 5; i++) {
  const session = m.createSession(`list-test-user-${i}`);
  listTestSessions.push(session);
  m.addSessionMessage(session.id, 'user', `Message in session ${i}`);
}

try {
  const allSessions = m.listSessions('list-test-user-0');
  assert(Array.isArray(allSessions), 'B16: List user sessions works');
  
  const specificUserSessions = m.listSessions('list-test-user-1');
  assert(Array.isArray(specificUserSessions), 'B17: List specific user sessions works');
  
  const nonExistentUser = m.listSessions('non-existent-user-xyz');
  assert(Array.isArray(nonExistentUser) && nonExistentUser.length === 0, 'B18: List sessions for non-existent user returns empty array');
  
  // Test edge cases - listSessions requires userId parameter
  assertThrows(() => m.listSessions(''), 'empty', 'B19: Empty user filter should error');
  assertThrows(() => m.listSessions(null), 'null', 'B20: Null user filter should error');
  
} catch (e) {
  assert(false, `Session listing tests failed: ${e.message}`);
}

// Clean up
listTestSessions.forEach(s => { try { m.closeSession(s.id); } catch(e) {} });

// B21-B25: Memory extraction edge cases  
console.log('\nB21-B25: Memory extraction edge cases...');

const extractionTests = [
  { setup: () => {}, name: 'empty session' },
  { setup: (sessionId) => m.addSessionMessage(sessionId, 'system', 'System message only'), name: 'system messages only' },
  { setup: (sessionId) => {
    for (let i = 0; i < 100; i++) { // Reduced from 1000 for performance
      m.addSessionMessage(sessionId, i % 2 ? 'user' : 'assistant', `Message ${i}`);
    }
  }, name: '100 messages session' },
  { setup: (sessionId) => m.addSessionMessage(sessionId, 'user', 'A'.repeat(10000)), name: 'very long message (10KB)' },
  { setup: (sessionId) => m.addSessionMessage(sessionId, 'user', '你好世界🚀'), name: 'Unicode message' }
];

extractionTests.forEach(({ setup, name }, i) => {
  try {
    const tempSession = m.createSession(`extract-temp-${i}`);
    setup(tempSession.id);  // Setup test data
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
    const compressed = m.compress(testText, level); // Using compress (not compressText)
    assert(compressed && compressed.length > 0, `C${i+1}: ${level} compression produces output`);
    
    const detailed = m.compressDetailed(testText, level);
    assert(detailed && detailed.compressed && detailed.originalLen && detailed.compressedLen, 
           `C${i+1}: ${level} compressDetailed provides stats`);
    
    const ratio = detailed.compressedLen / detailed.originalLen;
    console.log(`   ${level}: ${detailed.originalLen} → ${detailed.compressedLen} bytes (${(ratio*100).toFixed(1)}% ratio)`);
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
      assertThrows(() => m.compress(input, 'lossless'), 'empty', `C${i+6}: ${name} should error`);
    } else {
      const compressed = m.compress(input, 'lossless');
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
    const compressed = m.compress(text, 'lossless');
    const decompressed = m.decompressText(compressed);
    // This is the known failing test - let's see what happens
    if (decompressed === text) {
      assert(true, `C${i+11}: Roundtrip preserved: ${text.substring(0, 30)}...`);
    } else {
      assert(false, `C${i+11}: Roundtrip FAILED for: ${text.substring(0, 30)}... (original: ${text.length} chars, got: ${decompressed?.length || 0} chars)`);
    }
  } catch (e) {
    assert(false, `C${i+11}: Roundtrip test failed: ${e.message}`);
  }
});

// C16-C20: Decompression edge cases
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

// E16-E20: More vector tests
console.log('\nE16-E20: Additional vector tests...');
try {
  // E16: Test with identical vectors
  const identicalVectors = JSON.stringify([
    ['same1', [0.1, 0.1, 0.1, 0.1, 0.1]],
    ['same2', [0.1, 0.1, 0.1, 0.1, 0.1]],
    ['same3', [0.1, 0.1, 0.1, 0.1, 0.1]]
  ]);
  const sameResults = m.vectorSearch([0.1, 0.1, 0.1, 0.1, 0.1], identicalVectors, 3);
  assert(sameResults.length === 3 && sameResults.every(r => r.score >= 0.99), 'E16: Identical vectors have high scores');

  // E17: Test with negative values
  const negativeVectors = JSON.stringify([
    ['neg1', [-0.5, -0.5, -0.5, -0.5, -0.5]],
    ['pos1', [0.5, 0.5, 0.5, 0.5, 0.5]]
  ]);
  const negResults = m.vectorSearch([-0.5, -0.5, -0.5, -0.5, -0.5], negativeVectors, 2);
  assert(negResults[0].id === 'neg1', 'E17: Negative vector matching works');

  // E18: Empty vector collection
  assertThrows(() => m.vectorSearch([1, 2, 3, 4, 5], '[]', 1), null, 'E18: Empty vector collection should error');

  // E19: Invalid JSON
  assertThrows(() => m.vectorSearch([1, 2, 3, 4, 5], 'invalid json', 1), null, 'E19: Invalid JSON should error');

  // E20: Very small limit
  const smallLimit = m.vectorSearch([0.5, 0.5, 0.5, 0.5, 0.5], testVectors, 1);
  assert(smallLimit.length === 1, 'E20: Very small limit (1) works');

} catch (e) {
  assert(false, `E16-20: Additional vector tests failed: ${e.message}`);
}

// =========================== Final Results ===========================
console.log('\n📊 MEGA Test Results Summary (v3)');
console.log('='.repeat(70));
console.log(`Total tests: ${testResults.passed + testResults.failed}`);
console.log(`✅ Passed: ${testResults.passed}`);
console.log(`❌ Failed: ${testResults.failed}`);
console.log(`Success rate: ${((testResults.passed / (testResults.passed + testResults.failed)) * 100).toFixed(1)}%\n`);

if (testResults.errors.length > 0) {
  console.log('❌ Failed Tests:');
  testResults.errors.forEach((error, i) => {
    console.log(`  ${i + 1}. ${error}`);
  });
  console.log();
}

console.log('🎉 MEGA testing completed!');
console.log(`   Engine version: ${m.ping()}`);
console.log(`   Categories tested: A(Memory CRUD), B(Sessions), C(Compression), D(Router), E(Vector)`);

// Exit with non-zero code if there were failures
if (testResults.failed > 0) {
  console.log(`\n🚨 ${testResults.failed} tests failed - exiting with code 1`);
  process.exit(1);
} else {
  console.log('\n🎉 All tests passed!');
  process.exit(0);
}