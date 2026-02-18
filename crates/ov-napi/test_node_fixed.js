const path = require('path');

console.log('Testing OpenViking NAPI module...');

try {
    const ovModule = require('./openviking-engine.darwin-arm64.node');
    console.log('✓ Successfully loaded openviking-engine.darwin-arm64.node');
    
    // Test ping function
    const pingResult = ovModule.ping();
    console.log('ping() result:', pingResult);
    
    // Test createSession function
    if (ovModule.createSession) {
        const sessionResult = ovModule.createSession('test-user');
        console.log('createSession() result:', sessionResult);
    }
    
    // Test compress function
    if (ovModule.compress) {
        const compressResult = ovModule.compress('This is a test compression message', 'balanced');
        console.log('compress() result length:', compressResult.length);
    }
    
    // Test route function
    if (ovModule.route) {
        const routeResult = ovModule.route('What is 2+2?', 'auto');
        console.log('route() result:', routeResult);
    }
    
    console.log('✓ All tests passed!');
    
} catch (error) {
    console.error('✗ Error testing module:', error);
    process.exit(1);
}
