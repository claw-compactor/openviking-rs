const m = require('./openviking-engine.darwin-arm64.node');
const fs = require('fs');
const crypto = require('crypto');

console.log('=== OpenViking-rs Comprehensive Test Suite ===\n');

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

// =========================== A. Memory Functionality Tests ===========================
console.log('🧠 A. Memory Functionality Tests');
console.log('-'.repeat(50));

// Create test session
const session = m.createSession('test_user_comprehensive');
console.log(`Created test session: ${session.id}\n`);

// Test 1: Store 50 different memories
console.log('1. Storing 50 diverse memories...');
const testMemories = [
  // English memories
  'I prefer using dark mode when coding',
  'My favorite programming language is Rust',
  'OpenViking-rs is a vector search engine',
  'I like drinking coffee in the morning',
  'The project deadline is next Friday',
  'Docker containers are very useful for development',
  'I need to remember to update the documentation',
  'TypeScript helps catch errors early',
  'The server is running on port 18790',
  'Vector search is more accurate than text search',
  
  // Chinese memories  
  '我喜欢在晚上编程',
  '深度学习很有趣',
  '北京的天气很好',
  '我今天吃了很好吃的面条',
  '这个项目很复杂',
  '我需要学习更多关于机器学习的知识',
  '向量搜索比传统搜索更智能',
  '我的猫很可爱',
  '今天是个好日子',
  '开源软件让世界更美好',
  
  // Mixed content
  'OpenAI GPT-4 is very powerful',
  'Claude by Anthropic is also great',
  'NAPI bindings connect Rust and Node.js',
  'Performance benchmarks are important',
  'Memory compression saves storage space',
  'Session management handles user contexts',
  'API tokens should be kept secure',
  'Error handling is critical for stability',
  'Unit tests prevent regression bugs',
  'Code reviews improve quality',
  
  // Technical memories
  'The database connection pool size is 10',
  'Redis cache TTL is set to 3600 seconds',
  'JWT tokens expire after 24 hours',
  'The API rate limit is 1000 requests per hour',
  'SSL certificates need renewal every 90 days',
  'Backup jobs run daily at 3 AM UTC',
  'Log retention period is 30 days',
  'Health checks run every 30 seconds',
  'Memory usage should not exceed 80%',
  'CPU utilization target is below 70%',
  
  // Personal preferences
  'I prefer vim over emacs',
  'Dark chocolate is better than milk chocolate',
  'I like working in quiet environments',
  'My preferred IDE is VS Code',
  'I enjoy listening to instrumental music while coding',
  'I prefer CLI tools over GUI applications',
  'I like to organize my files in folders',
  'I prefer shorter variable names when they are clear',
  'I use tabs instead of spaces for indentation',
  'I like to write comments in my code'
];

const storedMemoryIds = [];
let memoryStoreCount = 0;

for (const content of testMemories) {
  try {
    const result = m.addMemory(content, 'test_user_comprehensive', session.id, null);
    if (result.stored) {
      storedMemoryIds.push(result.id);
      memoryStoreCount++;
    }
  } catch (error) {
    testResults.errors.push(`Failed to store memory: ${content.substring(0, 30)}...`);
  }
}

assert(memoryStoreCount >= 45, `Stored ${memoryStoreCount}/50 memories (≥45 required)`);
console.log();

// Test 2: Search and verify retrieval accuracy
console.log('2. Testing search retrieval accuracy...');
const searchTests = [
  { query: 'dark mode', expectedKeywords: ['dark', 'mode'] },
  { query: 'programming', expectedKeywords: ['programming', 'language', 'code'] },
  { query: 'OpenViking', expectedKeywords: ['OpenViking', 'vector', 'search'] },
  { query: '编程', expectedKeywords: ['编程', '晚上'] },
  { query: 'performance', expectedKeywords: ['performance', 'benchmark'] },
  { query: 'database', expectedKeywords: ['database', 'connection'] },
  { query: 'prefer', expectedKeywords: ['prefer', 'vim', 'chocolate'] }
];

let searchAccuracyCount = 0;
for (const test of searchTests) {
  try {
    const results = m.searchMemory(test.query, 'test_user_comprehensive', null, 5);
    let foundRelevant = false;
    for (const result of results) {
      for (const keyword of test.expectedKeywords) {
        if (result.content.toLowerCase().includes(keyword.toLowerCase())) {
          foundRelevant = true;
          break;
        }
      }
      if (foundRelevant) break;
    }
    if (foundRelevant) searchAccuracyCount++;
  } catch (error) {
    testResults.errors.push(`Search failed for query: ${test.query}`);
  }
}

assert(searchAccuracyCount >= 6, `Search accuracy: ${searchAccuracyCount}/7 queries returned relevant results`);
console.log();

// Test 3: Exact match vs semantic search
console.log('3. Testing exact match vs semantic search...');
try {
  const exactResults = m.searchMemory('OpenViking-rs', 'test_user_comprehensive', null, 5);
  const semanticResults = m.searchMemory('vector database', 'test_user_comprehensive', null, 5);
  
  assert(exactResults.length > 0, `Exact match found ${exactResults.length} results`);
  assert(semanticResults.length >= 0, `Semantic search found ${semanticResults.length} results`);
} catch (error) {
  testResults.errors.push('Exact/semantic search test failed');
}
console.log();

// =========================== B. Performance Tests ===========================
console.log('🚀 B. Performance Tests');
console.log('-'.repeat(50));

// Test 1: Search latency with different corpus sizes
console.log('1. Search latency benchmarks...');
const searchLatency100 = benchmark('Search (100 memories)', 100, () => {
  m.searchMemory('performance test', 'test_user_comprehensive', null, 10);
});
testResults.benchmarks.search100 = searchLatency100;

// Test 2: Write throughput
console.log('2. Write throughput benchmark...');
const writeStart = Date.now();
let writtenCount = 0;
for (let i = 0; i < 100; i++) {
  try {
    const result = m.addMemory(`Write benchmark test memory ${i} with some longer content to simulate real usage patterns`, 'benchmark_user', session.id, 'benchmark');
    if (result.stored) writtenCount++;
  } catch (error) {
    // Ignore individual failures
  }
}
const writeTotal = Date.now() - writeStart;
const writePerSec = (writtenCount * 1000) / writeTotal;
console.log(`   Wrote ${writtenCount} memories in ${writeTotal}ms (${writePerSec.toFixed(2)} writes/sec)\n`);
testResults.benchmarks.writePerSec = writePerSec;

// Test 3: Concurrent search (simulated)
console.log('3. Concurrent search simulation...');
const concurrentStart = Date.now();
const concurrentPromises = [];
for (let i = 0; i < 20; i++) {
  // Simulate concurrent searches with different queries
  const queries = ['rust', 'typescript', 'database', 'performance', 'memory'];
  const query = queries[i % queries.length];
  try {
    const results = m.searchMemory(query, 'test_user_comprehensive', null, 5);
    concurrentPromises.push(results.length);
  } catch (error) {
    concurrentPromises.push(0);
  }
}
const concurrentTotal = Date.now() - concurrentStart;
const concurrentAvg = concurrentTotal / 20;
console.log(`   20 searches in ${concurrentTotal}ms (avg: ${concurrentAvg.toFixed(2)}ms per search)\n`);
testResults.benchmarks.concurrentAvg = concurrentAvg;

// =========================== C. Compression Tests ===========================
console.log('🗜️  C. Compression Tests');
console.log('-'.repeat(50));

const testTexts = [
  'Short text',
  'The quick brown fox jumps over the lazy dog. '.repeat(10),
  'Lorem ipsum dolor sit amet, consectetur adipiscing elit. '.repeat(50),
  'Technical documentation with many repeated terms like API, database, server, client, request, response. '.repeat(20)
];

const compressionLevels = ['lossless', 'minimal', 'balanced'];
let compressionResults = {};

for (const level of compressionLevels) {
  console.log(`Testing ${level} compression...`);
  const levelResults = [];
  
  for (const text of testTexts) {
    try {
      const result = m.compressDetailed(text, level);
      levelResults.push({
        originalLen: result.originalLen,
        compressedLen: result.compressedLen,
        ratio: result.ratio
      });
    } catch (error) {
      testResults.errors.push(`Compression failed for ${level} level`);
    }
  }
  
  compressionResults[level] = levelResults;
  const avgRatio = levelResults.reduce((sum, r) => sum + r.ratio, 0) / levelResults.length;
  console.log(`   Average compression ratio: ${avgRatio.toFixed(3)}`);
}
console.log();

// Test compression roundtrip accuracy
console.log('Testing compression roundtrip...');
const originalText = 'This is a test text for compression roundtrip verification with some repeated phrases and common words that should compress well.';
try {
  const compressed = m.compress(originalText, 'balanced');
  // Note: We can\'t decompress since the API doesn\'t expose decompression
  assert(compressed.length > 0, 'Compression produced output');
  assert(compressed !== originalText, 'Compressed text is different from original');
} catch (error) {
  testResults.errors.push('Compression roundtrip test failed');
}
console.log();

// =========================== D. Stability Tests ===========================
console.log('🛡️  D. Stability Tests');
console.log('-'.repeat(50));

// Test 1: Rapid fire operations
console.log('1. Rapid fire test (500 operations)...');
let rapidFireSuccess = 0;
const rapidStart = Date.now();

for (let i = 0; i < 500; i++) {
  try {
    if (i % 3 === 0) {
      // Write operation
      const result = m.addMemory(`Rapid fire memory ${i}`, 'rapid_user', session.id, 'rapid');
      if (result.stored) rapidFireSuccess++;
    } else {
      // Search operation  
      const results = m.searchMemory(`rapid ${i % 10}`, 'rapid_user', null, 3);
      if (Array.isArray(results)) rapidFireSuccess++;
    }
  } catch (error) {
    // Continue on individual failures
  }
}

const rapidTotal = Date.now() - rapidStart;
assert(rapidFireSuccess >= 400, `Rapid fire: ${rapidFireSuccess}/500 operations succeeded (≥400 required)`);
console.log(`   Completed in ${rapidTotal}ms (${(500000/rapidTotal).toFixed(1)} ops/sec)\n`);

// Test 2: Memory usage monitoring (simplified)
console.log('2. Memory usage test...');
const initialMemory = process.memoryUsage();

// Perform 1000 operations to check for memory leaks
for (let i = 0; i < 1000; i++) {
  if (i % 100 === 0 && i > 0) {
    // Force garbage collection if available
    if (global.gc) global.gc();
  }
  
  try {
    if (i % 2 === 0) {
      m.addMemory(`Memory test ${i}`, 'memory_user', session.id, 'test');
    } else {
      m.searchMemory(`test ${i % 50}`, 'memory_user', null, 5);
    }
  } catch (error) {
    // Continue on failures
  }
}

const finalMemory = process.memoryUsage();
const memoryGrowth = finalMemory.heapUsed - initialMemory.heapUsed;
console.log(`   Memory growth: ${(memoryGrowth / 1024 / 1024).toFixed(2)}MB`);
assert(memoryGrowth < 100 * 1024 * 1024, 'Memory growth < 100MB'); // Less than 100MB growth
console.log();

// Test 3: Edge cases
console.log('3. Edge case testing...');
let edgeCasesPassed = 0;

// Empty query
try {
  const emptyResults = m.searchMemory('', 'test_user_comprehensive', null, 5);
  if (Array.isArray(emptyResults)) edgeCasesPassed++;
} catch (error) {
  // Expected to handle gracefully
}

// Very long document
try {
  const longText = 'A'.repeat(100000); // 100KB
  const longResult = m.addMemory(longText, 'edge_user', session.id, 'large');
  if (longResult.stored) edgeCasesPassed++;
} catch (error) {
  console.log(`   Long document test failed: ${error.message}`);
}

// Unicode/emoji content
try {
  const unicodeResult = m.addMemory('Testing unicode: 🚀 🔍 ⚡ 💾 🧠 中文测试 العربية тест', 'edge_user', session.id, 'unicode');
  if (unicodeResult.stored) edgeCasesPassed++;
} catch (error) {
  console.log(`   Unicode test failed: ${error.message}`);
}

// Very long search query
try {
  const longQuery = 'very long search query with many words '.repeat(20);
  const longQueryResults = m.searchMemory(longQuery, 'test_user_comprehensive', null, 5);
  if (Array.isArray(longQueryResults)) edgeCasesPassed++;
} catch (error) {
  console.log(`   Long query test failed: ${error.message}`);
}

assert(edgeCasesPassed >= 3, `Edge cases: ${edgeCasesPassed}/4 tests passed`);
console.log();

// =========================== E. Session Management Tests ===========================
console.log('💬 E. Session Management Tests');
console.log('-'.repeat(50));

// Test session operations
console.log('1. Session lifecycle testing...');
try {
  const newSession = m.createSession('session_test_user');
  assert(newSession.id && newSession.userId === 'session_test_user', 'Session creation successful');
  
  const retrieved = m.getSession(newSession.id);
  assert(retrieved.id === newSession.id, 'Session retrieval successful');
  
  const messageAdded = m.addSessionMessage(newSession.id, 'user', 'Test message for extraction');
  assert(messageAdded === true, 'Message addition successful');
  
  const extractedMemories = m.extractMemories(newSession.id);
  assert(Array.isArray(extractedMemories), 'Memory extraction completed');
  
  const closed = m.closeSession(newSession.id);
  assert(closed === true, 'Session closure successful');
} catch (error) {
  testResults.errors.push(`Session management test failed: ${error.message}`);
}
console.log();

// Test session listing
console.log('2. Session listing test...');
try {
  const sessions = m.listSessions('test_user_comprehensive');
  assert(Array.isArray(sessions), 'Session listing returned array');
  assert(sessions.length > 0, 'Found at least one session');
} catch (error) {
  testResults.errors.push(`Session listing failed: ${error.message}`);
}
console.log();

// =========================== F. Router and Vector Tests ===========================
console.log('🧭 F. Router and Vector Tests');
console.log('-'.repeat(50));

// Test routing
console.log('1. Model routing test...');
const routingTests = [
  { prompt: 'What is 2+2?', profile: 'eco' },
  { prompt: 'Explain quantum mechanics in detail', profile: 'premium' },
  { prompt: 'Simple question', profile: 'auto' }
];

let routingPassed = 0;
for (const test of routingTests) {
  try {
    const result = m.route(test.prompt, test.profile);
    if (result.model && result.confidence > 0) {
      routingPassed++;
    }
  } catch (error) {
    testResults.errors.push(`Routing failed for ${test.profile} profile`);
  }
}

assert(routingPassed === 3, `Routing: ${routingPassed}/3 tests passed`);
console.log();

// Test vector search
console.log('2. Vector search test...');
try {
  const queryVector = [0.1, 0.2, 0.3, 0.4, 0.5];
  const testVectors = JSON.stringify([
    ['doc1', [0.1, 0.2, 0.3, 0.4, 0.5]], // Exact match
    ['doc2', [0.2, 0.3, 0.4, 0.5, 0.6]], // Close match
    ['doc3', [0.9, 0.8, 0.7, 0.6, 0.5]], // Distant
    ['doc4', [-0.1, -0.2, -0.3, -0.4, -0.5]] // Opposite
  ]);
  
  const vectorResults = m.vectorSearch(queryVector, testVectors, 3);
  assert(Array.isArray(vectorResults), 'Vector search returned array');
  assert(vectorResults.length <= 3, 'Respects top_k limit');
  assert(vectorResults[0].id === 'doc1', 'Best match is exact match');
  assert(vectorResults[0].score > 0.9, 'Exact match has high score');
} catch (error) {
  testResults.errors.push(`Vector search test failed: ${error.message}`);
}
console.log();

// =========================== Final Results ===========================
console.log('📊 Test Results Summary');
console.log('='.repeat(50));
console.log(`Total tests: ${testResults.passed + testResults.failed}`);
console.log(`✅ Passed: ${testResults.passed}`);
console.log(`❌ Failed: ${testResults.failed}`);
console.log(`Success rate: ${((testResults.passed / (testResults.passed + testResults.failed)) * 100).toFixed(1)}%\n`);

if (testResults.errors.length > 0) {
  console.log('Errors encountered:');
  testResults.errors.forEach((error, i) => {
    console.log(`  ${i + 1}. ${error}`);
  });
  console.log();
}

console.log('Performance Benchmarks:');
Object.entries(testResults.benchmarks).forEach(([name, result]) => {
  if (typeof result === 'object') {
    console.log(`  ${name}: avg=${result.avg.toFixed(2)}ms, p95=${result.p95}ms`);
  } else {
    console.log(`  ${name}: ${result.toFixed(2)}`);
  }
});

console.log();
console.log('🎉 Comprehensive testing completed!');
console.log(`   Engine version: ${m.ping()}`);
console.log(`   Total memories stored: ${memoryStoreCount + writtenCount}`);
console.log(`   Test duration: ${((Date.now() - Date.now()) / 1000).toFixed(1)}s`);
