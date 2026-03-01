#!/usr/bin/env python3
"""
Mock Bond Server for testing HLX Bond Protocol (Phase 12)

Runs a minimal HTTP server that implements the /bond and /infer endpoints
required by the Bond Protocol handshake.

Usage:
    python3 mock_bond_server.py [PORT]

Default port is 8765. Set HLX_BOND_ENDPOINT=http://localhost:8765 when running hlx-run.
"""

import json
import sys
from http.server import HTTPServer, BaseHTTPRequestHandler


class MockBondHandler(BaseHTTPRequestHandler):
    """Handler for Bond Protocol endpoints."""
    
    def log_message(self, format, *args):
        """Suppress default logging - we'll print our own."""
        pass
    
    def do_POST(self):
        """Handle POST requests to /bond and /infer."""
        content_length = int(self.headers.get('Content-Length', 0))
        body = self.rfile.read(content_length).decode('utf-8')
        
        try:
            data = json.loads(body) if body else {}
        except json.JSONDecodeError:
            self._send_error(400, "Invalid JSON")
            return
        
        if self.path == '/bond':
            self._handle_bond(data)
        elif self.path == '/infer':
            self._handle_infer(data)
        else:
            self._send_error(404, f"Unknown endpoint: {self.path}")
    
    def _handle_bond(self, data):
        """Handle BondRequest and return BondResponse."""
        print(f"[MockBond] Received bond request from symbiote")
        print(f"[MockBond] Protocol version: {data.get('protocol_version', 'unknown')}")
        print(f"[MockBond] Capabilities: {[c.get('name') for c in data.get('capabilities', [])]}")
        
        response = {
            "accepted": True,
            "model_name": "mock-llm",
            "model_version": "1.0.0",
            "context_window": 4096,
            "capabilities": [
                {"name": "text_generation", "version": "1.0", "description": "Generate text responses"},
                {"name": "reasoning", "version": "1.0", "description": "Basic reasoning capabilities"},
            ],
            "rejection_reason": None,
        }
        
        self._send_json(200, response)
        print("[MockBond] Bond accepted, sent BondResponse")
    
    def _handle_infer(self, data):
        """Handle inference request and return generated response."""
        prompt = data.get('prompt', '')
        symbiote_id = data.get('symbiote_id', 'unknown')
        
        print(f"[MockBond] Inference request from symbiote {symbiote_id[:8]}...")
        print(f"[MockBond] Prompt: {prompt[:60]}...")
        
        # Generate a mock response
        response = {
            "response": f"Mock LLM response to: {prompt}",
            "model": "mock-llm",
            "tokens_used": len(prompt.split()),
        }
        
        self._send_json(200, response)
        print(f"[MockBond] Sent inference response")
    
    def _send_json(self, status_code, data):
        """Send JSON response."""
        self.send_response(status_code)
        self.send_header('Content-Type', 'application/json')
        self.end_headers()
        self.wfile.write(json.dumps(data).encode('utf-8'))
    
    def _send_error(self, status_code, message):
        """Send error response."""
        self.send_response(status_code)
        self.send_header('Content-Type', 'application/json')
        self.end_headers()
        self.wfile.write(json.dumps({"error": message}).encode('utf-8'))


def main():
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8765
    server = HTTPServer(('localhost', port), MockBondHandler)
    
    print(f"╔══════════════════════════════════════════════════════════════╗")
    print(f"║           Mock Bond Server (HLX Test Infrastructure)         ║")
    print(f"╠══════════════════════════════════════════════════════════════╣")
    print(f"║  Listening on: http://localhost:{port}                     ║")
    print(f"║  Endpoints:                                                  ║")
    print(f"║    POST /bond  - BondProtocol handshake                      ║")
    print(f"║    POST /infer - LLM inference                               ║")
    print(f"╠══════════════════════════════════════════════════════════════╣")
    print(f"║  Usage:                                                      ║")
    print(f"║    HLX_BOND_ENDPOINT=http://localhost:{port} \\\           ║")
    print(f"║      cargo run -p hlx-run -- <program.hlx>                  ║")
    print(f"╚══════════════════════════════════════════════════════════════╝")
    print()
    
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n[MockBond] Shutting down...")
        server.shutdown()


if __name__ == '__main__':
    main()
