const path = require('path');

console.log('Testing OpenViking NAPI module...');

// Try loading the existing .node file first
try {
    const ovModule = require('./openviking-engine.darwin-arm64.node');
    console.log('✓ Successfully loaded openviking-engine.darwin-arm64.node');
    
    // Test ping function
    const pingResult = ovModule.ping();
    console.log('ping() result:', pingResult);
    
    // Test add_memory function
    const memoryResult = ovModule.addMemory('I love Node.js integration tests', 'test-user', null, null);
    console.log('addMemory() result:', memoryResult);
    
    // Test search_memory function
    const searchResult = ovModule.searchMemory('Node.js', 'test-user', null, null);
    console.log('searchMemory() result:', searchResult);
    
    console.log('✓ All tests passed!');
    
} catch (error) {
    console.error('✗ Error testing module:', error);
    process.exit(1);
}
