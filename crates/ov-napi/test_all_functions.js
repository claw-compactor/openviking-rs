const m = require('./openviking-engine.darwin-arm64.node');

console.log('=== OpenViking-rs NAPI Function Test ===\n');

// Test 1: Ping
console.log('1. Testing ping()');
const pingResult = m.ping();
console.log('Result:', pingResult);
if (pingResult === 'openviking-rs v0.1.0') {
    console.log('✅ PASS\n');
} else {
    console.log('❌ FAIL\n');
}

// Test 2: Create Session
console.log('2. Testing createSession()');
const session = m.createSession('test_user_123');
console.log('Result:', session);
if (session.id && session.user_id === 'test_user_123') {
    console.log('✅ PASS\n');
} else {
    console.log('❌ FAIL\n');
}

// Test 3: Add Memory
console.log('3. Testing addMemory()');
const memory1 = m.addMemory('I prefer dark mode in my IDE', 'test_user_123', session.id, 'preferences');
console.log('Result:', memory1);
if (memory1.id && memory1.category === 'preferences' && memory1.stored) {
    console.log('✅ PASS\n');
} else {
    console.log('❌ FAIL\n');
}

// Test 4: Add more memories for search testing
const memory2 = m.addMemory('The project name is OpenViking-rs', 'test_user_123', session.id, 'entities');
const memory3 = m.addMemory('I found a bug in the vector search', 'test_user_123', session.id, 'cases');
const memory4 = m.addMemory('My name is Duke Nukem', 'test_user_123', session.id, 'profile');

// Test 5: Search Memory
console.log('4. Testing searchMemory()');
const searchResults = m.searchMemory('OpenViking', 'test_user_123', null, 10);
console.log('Search results for "OpenViking":', searchResults);
if (Array.isArray(searchResults) && searchResults.length > 0) {
    console.log('✅ PASS\n');
} else {
    console.log('❌ FAIL\n');
}

// Test 6: Compression
console.log('5. Testing compress()');
const compressed = m.compress('This is a long text that should be compressed to save space and reduce tokens.', 'balanced');
console.log('Compressed text length:', compressed.length);
if (compressed && compressed.length > 0) {
    console.log('✅ PASS\n');
} else {
    console.log('❌ FAIL\n');
}

// Test 7: Detailed Compression
console.log('6. Testing compressDetailed()');
const text = 'The quick brown fox jumps over the lazy dog. '.repeat(10);
const detailed = m.compressDetailed(text, 'balanced');
console.log('Compression details:', detailed);
if (detailed.ratio && detailed.original_len > 0 && detailed.compressed_len > 0) {
    console.log('✅ PASS\n');
} else {
    console.log('❌ FAIL\n');
}

// Test 8: Route
console.log('7. Testing route()');
const routeResult = m.route('What is the capital of France?', 'auto');
console.log('Routing result:', routeResult);
if (routeResult.model && routeResult.confidence > 0) {
    console.log('✅ PASS\n');
} else {
    console.log('❌ FAIL\n');
}

// Test 9: Vector Search
console.log('8. Testing vectorSearch()');
const query = [0.1, 0.2, 0.3, 0.4, 0.5];
const vectors = JSON.stringify([
    ['vec1', [0.1, 0.2, 0.3, 0.4, 0.5]],
    ['vec2', [0.2, 0.3, 0.4, 0.5, 0.6]],
    ['vec3', [0.9, 0.8, 0.7, 0.6, 0.5]]
]);
const vectorResults = m.vectorSearch(query, vectors, 2);
console.log('Vector search results:', vectorResults);
if (Array.isArray(vectorResults) && vectorResults.length > 0) {
    console.log('✅ PASS\n');
} else {
    console.log('❌ FAIL\n');
}

// Test 10: Add Session Message
console.log('9. Testing addSessionMessage()');
const messageAdded = m.addSessionMessage(session.id, 'user', 'Hello, this is a test message');
console.log('Message added:', messageAdded);
if (messageAdded === true) {
    console.log('✅ PASS\n');
} else {
    console.log('❌ FAIL\n');
}

// Test 11: Extract Memories
console.log('10. Testing extractMemories()');
const extracted = m.extractMemories(session.id);
console.log('Extracted memories:', extracted);
if (Array.isArray(extracted)) {
    console.log('✅ PASS\n');
} else {
    console.log('❌ FAIL\n');
}

// Test 12: Get Session
console.log('11. Testing getSession()');
const retrievedSession = m.getSession(session.id);
console.log('Retrieved session:', retrievedSession);
if (retrievedSession.id === session.id) {
    console.log('✅ PASS\n');
} else {
    console.log('❌ FAIL\n');
}

// Test 13: List Sessions
console.log('12. Testing listSessions()');
const sessions = m.listSessions('test_user_123');
console.log('Sessions:', sessions);
if (Array.isArray(sessions) && sessions.length > 0) {
    console.log('✅ PASS\n');
} else {
    console.log('❌ FAIL\n');
}

// Test 14: Close Session
console.log('13. Testing closeSession()');
const closed = m.closeSession(session.id);
console.log('Session closed:', closed);
if (closed === true) {
    console.log('✅ PASS\n');
} else {
    console.log('❌ FAIL\n');
}

console.log('=== All NAPI Function Tests Completed ===');
