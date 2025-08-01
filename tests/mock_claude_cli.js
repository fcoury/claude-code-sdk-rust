#!/usr/bin/env node

/**
 * Mock Claude Code CLI for integration testing
 * 
 * This script simulates the behavior of the real claude-code CLI for testing purposes.
 * It supports various test scenarios including:
 * - Normal message flow
 * - JSON buffering scenarios (split/concatenated messages)
 * - Error conditions
 * - Process lifecycle testing
 * - Control/interrupt handling
 */

const fs = require('fs');
const path = require('path');

// Parse command line arguments
const args = process.argv.slice(2);
let mode = 'normal';
let delay = 0;
let streaming = false;
let exitCode = 0;
let errorMessage = '';

// Parse arguments
for (let i = 0; i < args.length; i++) {
    switch (args[i]) {
        case '--test-mode':
            mode = args[i + 1];
            i++;
            break;
        case '--test-delay':
            delay = parseInt(args[i + 1]);
            i++;
            break;
        case '--streaming':
            streaming = true;
            break;
        case '--test-exit-code':
            exitCode = parseInt(args[i + 1]);
            i++;
            break;
        case '--test-error-message':
            errorMessage = args[i + 1];
            i++;
            break;
    }
}

// Helper function to write JSON message
function writeMessage(message) {
    console.log(JSON.stringify(message));
}

// Helper function to write partial JSON (for buffering tests)
function writePartialJson(json, parts) {
    const jsonStr = JSON.stringify(json);
    const chunkSize = Math.ceil(jsonStr.length / parts);
    
    for (let i = 0; i < jsonStr.length; i += chunkSize) {
        process.stdout.write(jsonStr.slice(i, i + chunkSize));
        if (delay > 0 && i + chunkSize < jsonStr.length) {
            // In a real scenario, we'd use setTimeout, but for testing we'll just continue
        }
    }
    process.stdout.write('\n');
}

// Helper function to simulate delay
function sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

// Helper function to wait for stdin input (simulating real CLI behavior)
function waitForStdinInput() {
    return new Promise((resolve) => {
        process.stdin.setEncoding('utf8');
        let inputReceived = false;
        
        const timeout = setTimeout(() => {
            if (!inputReceived) {
                resolve(); // Resolve anyway after timeout
            }
        }, 1000); // 1 second timeout
        
        process.stdin.once('data', (data) => {
            inputReceived = true;
            clearTimeout(timeout);
            resolve();
        });
        
        process.stdin.once('end', () => {
            inputReceived = true;
            clearTimeout(timeout);
            resolve();
        });
    });
}

// Main execution
async function main() {
    // Handle different test modes
    switch (mode) {
        case 'normal':
            await normalFlow();
            break;
        case 'split_json':
            await splitJsonFlow();
            break;
        case 'concatenated_json':
            await concatenatedJsonFlow();
            break;
        case 'large_message':
            await largeMessageFlow();
            break;
        case 'error_exit':
            await errorExitFlow();
            break;
        case 'stderr_output':
            await stderrOutputFlow();
            break;
        case 'interactive':
            await interactiveFlow();
            break;
        case 'control_interrupt':
            await controlInterruptFlow();
            break;
        case 'unicode_content':
            await unicodeContentFlow();
            break;
        case 'empty_response':
            await emptyResponseFlow();
            break;
        case 'malformed_json':
            await malformedJsonFlow();
            break;
        default:
            console.error(`Unknown test mode: ${mode}`);
            process.exit(1);
    }
    
    process.exit(exitCode);
}

async function normalFlow() {
    // Wait for stdin input first (simulating real CLI behavior)
    await waitForStdinInput();
    
    if (delay > 0) await sleep(delay);
    
    // System message
    writeMessage({
        type: "system",
        subtype: "session_start",
        data: {
            session_id: "test_session_123",
            timestamp: Date.now()
        }
    });
    
    if (delay > 0) await sleep(delay);
    
    // Assistant response
    writeMessage({
        type: "assistant",
        content: [
            {
                type: "text",
                text: "Hello! This is a test response from the mock CLI."
            }
        ]
    });
    
    if (delay > 0) await sleep(delay);
    
    // Result message
    writeMessage({
        type: "result",
        subtype: "query_complete",
        duration_ms: 1500,
        duration_api_ms: 1200,
        is_error: false,
        num_turns: 1,
        session_id: "test_session_123",
        total_cost_usd: 0.01,
        usage: {
            input_tokens: 50,
            output_tokens: 25
        },
        result: "Query completed successfully"
    });
}

async function splitJsonFlow() {
    // Wait for stdin input first
    await waitForStdinInput();
    
    // Send a message split across multiple writes to test JSON buffering
    const message = {
        type: "assistant",
        content: [
            {
                type: "text",
                text: "This is a very long message that will be split across multiple writes to test the JSON buffering capabilities of the transport layer. ".repeat(10)
            }
        ]
    };
    
    writePartialJson(message, 5); // Split into 5 parts
    
    if (delay > 0) await sleep(delay);
    
    // Result message
    writeMessage({
        type: "result",
        subtype: "query_complete",
        duration_ms: 800,
        duration_api_ms: 600,
        is_error: false,
        num_turns: 1,
        session_id: "split_test_session"
    });
}

async function concatenatedJsonFlow() {
    // Wait for stdin input first
    await waitForStdinInput();
    
    // Send multiple JSON messages concatenated together
    const messages = [
        {
            type: "system",
            subtype: "session_start",
            data: { session_id: "concat_test" }
        },
        {
            type: "assistant",
            content: [{ type: "text", text: "First message" }]
        },
        {
            type: "assistant", 
            content: [{ type: "text", text: "Second message" }]
        }
    ];
    
    // Write all messages as one concatenated string
    const concatenated = messages.map(m => JSON.stringify(m)).join('\n') + '\n';
    process.stdout.write(concatenated);
    
    if (delay > 0) await sleep(delay);
    
    writeMessage({
        type: "result",
        subtype: "query_complete",
        duration_ms: 500,
        duration_api_ms: 400,
        is_error: false,
        num_turns: 1,
        session_id: "concat_test"
    });
}

async function largeMessageFlow() {
    // Wait for stdin input first
    await waitForStdinInput();
    
    // Send a very large message to test buffer limits
    const largeText = "x".repeat(5000); // 5KB of text
    
    writeMessage({
        type: "assistant",
        content: [
            {
                type: "text",
                text: largeText
            },
            {
                type: "tool_use",
                id: "large_tool_123",
                name: "data_processor",
                input: {
                    large_data: "y".repeat(2500),
                    metadata: {
                        size: 2500,
                        type: "test_data"
                    }
                }
            }
        ]
    });
    
    writeMessage({
        type: "result",
        subtype: "query_complete",
        duration_ms: 2000,
        duration_api_ms: 1800,
        is_error: false,
        num_turns: 1,
        session_id: "large_test"
    });
}

async function errorExitFlow() {
    // Small delay to let the transport set up streams, then exit with error
    await sleep(50);
    // Write error to stderr and exit with non-zero code
    console.error(errorMessage || "Mock CLI error for testing");
    // Force immediate exit
    process.exitCode = exitCode || 1;
    process.exit(exitCode || 1);
}

async function stderrOutputFlow() {
    // Wait for stdin input first
    await waitForStdinInput();
    
    // Write to stderr but continue normally
    console.error("Warning: This is a test stderr message");
    console.error("Debug: Processing test request");
    
    if (delay > 0) await sleep(delay);
    
    // System message
    writeMessage({
        type: "system",
        subtype: "session_start",
        data: {
            session_id: "stderr_test_123",
            timestamp: Date.now()
        }
    });
    
    if (delay > 0) await sleep(delay);
    
    // Assistant response
    writeMessage({
        type: "assistant",
        content: [
            {
                type: "text",
                text: "Hello! This is a test response with stderr output."
            }
        ]
    });
    
    if (delay > 0) await sleep(delay);
    
    // Result message
    writeMessage({
        type: "result",
        subtype: "query_complete",
        duration_ms: 1500,
        duration_api_ms: 1200,
        is_error: false,
        num_turns: 1,
        session_id: "stderr_test_123",
        result: "Query completed successfully with stderr"
    });
}

async function interactiveFlow() {
    // Simulate interactive mode - read from stdin and respond
    process.stdin.setEncoding('utf8');
    
    writeMessage({
        type: "system",
        subtype: "session_start", 
        data: { session_id: "interactive_test", mode: "interactive" }
    });
    
    let messageCount = 0;
    
    process.stdin.on('data', async (data) => {
        try {
            const lines = data.trim().split('\n');
            for (const line of lines) {
                if (line.trim()) {
                    const message = JSON.parse(line);
                    messageCount++;
                    
                    // Echo back a response
                    writeMessage({
                        type: "assistant",
                        content: [
                            {
                                type: "text",
                                text: `Interactive response ${messageCount} to: ${JSON.stringify(message)}`
                            }
                        ]
                    });
                    
                    if (delay > 0) await sleep(delay);
                    
                    // Send result after each message
                    writeMessage({
                        type: "result",
                        subtype: "turn_complete",
                        duration_ms: 300,
                        duration_api_ms: 250,
                        is_error: false,
                        num_turns: messageCount,
                        session_id: "interactive_test"
                    });
                }
            }
        } catch (e) {
            console.error(`Error parsing input: ${e.message}`);
        }
    });
    
    process.stdin.on('end', () => {
        writeMessage({
            type: "result",
            subtype: "session_end",
            duration_ms: messageCount * 300,
            duration_api_ms: messageCount * 250,
            is_error: false,
            num_turns: messageCount,
            session_id: "interactive_test"
        });
        process.exit(0);
    });
}

async function controlInterruptFlow() {
    // Simulate handling control/interrupt requests
    writeMessage({
        type: "system",
        subtype: "session_start",
        data: { session_id: "control_test" }
    });
    
    // Start a long-running operation
    writeMessage({
        type: "assistant",
        content: [{ type: "text", text: "Starting long operation..." }]
    });
    
    // Listen for interrupt signals
    process.on('SIGINT', () => {
        writeMessage({
            type: "system",
            subtype: "interrupted",
            data: { reason: "user_interrupt" }
        });
        
        writeMessage({
            type: "result",
            subtype: "interrupted",
            duration_ms: 1000,
            duration_api_ms: 800,
            is_error: true,
            num_turns: 1,
            session_id: "control_test"
        });
        
        process.exit(0);
    });
    
    // Simulate long operation
    for (let i = 0; i < 10; i++) {
        await sleep(500);
        writeMessage({
            type: "assistant",
            content: [{ type: "text", text: `Progress: ${i + 1}/10` }]
        });
    }
    
    writeMessage({
        type: "result",
        subtype: "query_complete",
        duration_ms: 5000,
        duration_api_ms: 4500,
        is_error: false,
        num_turns: 1,
        session_id: "control_test"
    });
}

async function unicodeContentFlow() {
    // Wait for stdin input first
    await waitForStdinInput();
    
    writeMessage({
        type: "assistant",
        content: [
            {
                type: "text",
                text: "Unicode test: Hello 世界! 🌍 Здравствуй мир! مرحبا بالعالم! こんにちは世界！"
            },
            {
                type: "tool_use",
                id: "unicode_tool",
                name: "text_processor",
                input: {
                    text: "Processing unicode: 测试数据 🚀",
                    language: "多语言",
                    emoji: "🎉🔥💯"
                }
            }
        ]
    });
    
    writeMessage({
        type: "result",
        subtype: "query_complete",
        duration_ms: 800,
        duration_api_ms: 600,
        is_error: false,
        num_turns: 1,
        session_id: "unicode_test"
    });
}

async function emptyResponseFlow() {
    // Wait for stdin input first
    await waitForStdinInput();
    
    // Send minimal/empty responses
    writeMessage({
        type: "assistant",
        content: []
    });
    
    writeMessage({
        type: "result",
        subtype: "query_complete",
        duration_ms: 100,
        duration_api_ms: 50,
        is_error: false,
        num_turns: 1,
        session_id: "empty_test"
    });
}

async function malformedJsonFlow() {
    // Wait for stdin input first
    await waitForStdinInput();
    
    // Send malformed JSON to test error handling
    process.stdout.write('{"type": "assistant", "content": [{"type": "text", "text": "incomplete\n');
    await sleep(100);
    process.stdout.write('this is not json at all\n');
    await sleep(100);
    process.stdout.write('{"another": "broken" json}\n');
    await sleep(100);
    
    // Follow with valid message
    writeMessage({
        type: "result",
        subtype: "query_complete",
        duration_ms: 200,
        duration_api_ms: 150,
        is_error: false,
        num_turns: 1,
        session_id: "malformed_test"
    });
}

// Run the main function
main().catch(error => {
    console.error('Mock CLI error:', error);
    process.exit(1);
});