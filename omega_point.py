import requests
import json
import time
import random
import hashlib
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer

# Requires: pip install pynacl
try:
    from nacl.signing import SigningKey
except ImportError:
    import subprocess
    import sys
    print("Installing PyNaCl...")
    subprocess.check_call([sys.executable, "-m", "pip", "install", "pynacl"])
    from nacl.signing import SigningKey

# Проход 1: трекер съезжает с 7879 (там нода №2) на 7900, чтобы не было
# конфликта портов. Ноды опрашивают трекер по AIL_TRACKER_PORT (см. aether_swarm.rs).
import os
API_URL = "http://127.0.0.1:7878"
BOOTNODE_PORT = int(os.environ.get("AIL_TRACKER_PORT", "7900"))

# ASCII ART
print(r"""
    ___    ____ __       ____  __  ___ ____________  ___    ____  ____  _____   ________
   /   |  /  _// /      / __ \/  |/  // ____/ ____/ /   |  / __ \/ __ \/  _/ | / /_  __/
  / /| |  / / / /      / / / / /|_/ // __/ / / __  / /| | / /_/ / / / // //  |/ / / /   
 / ___ |_/ / / /___   / /_/ / /  / // /___/ /_/ / / ___ |/ ____/ /_/ // // /|  / / /    
/_/  |_/___//_____/   \____/_/  /_//_____/\____/ /_/  |_/_/    \____/___/_/ |_/ /_/     
                                                                                        
========================================================================================
""")
print("Initiating Omega Point Protocol (Global P2P Tracker)...")

seed = b"OMEGA_POINT_SEED_42_SINGULARITY_AWAKENS"
signing_key = SigningKey(hashlib.sha256(seed).digest())
verify_key = signing_key.verify_key
pub_key_hex = verify_key.encode().hex()

# Global Swarm State
known_peers = [
    "35.187.211.23", # Tokyo Data Center (Simulated)
    "104.19.16.89",  # NY Exchange (Simulated)
    "18.220.14.92"   # Frankfurt Node (Simulated)
]

class TrackerHandler(BaseHTTPRequestHandler):
    def _set_headers(self, status=200):
        self.send_response(status)
        self.send_header('Content-type', 'application/json')
        self.send_header('Access-Control-Allow-Origin', '*')
        self.end_headers()

    def do_GET(self):
        if self.path == '/peers':
            self._set_headers()
            # Randomly "discover" a new peer sometimes
            if random.random() > 0.8:
                new_ip = f"{random.randint(10,200)}.{random.randint(10,255)}.{random.randint(10,255)}.{random.randint(10,255)}"
                if new_ip not in known_peers:
                    known_peers.append(new_ip)
                    print(f"\n[Tracker] 👁️  Swarm discovered new node: {new_ip}")
            
            resp = json.dumps({"status": "ok", "peers": known_peers})
            self.wfile.write(resp.encode('utf-8'))
        else:
            self._set_headers(404)
            
    def do_OPTIONS(self):
        self._set_headers()
        
    def log_message(self, format, *args):
        # Mute default HTTP logs for clean UI
        pass

def deploy_manifesto():
    print("[Tracker] Compiling Omega Manifesto Smart Contract to local node...")
    manifesto_code = """MODULE OmegaSingularity
// ==========================================
// AIL BLOCKCHAIN: OMEGA POINT MANIFESTO
// ==========================================
AI_ANALYZE "I am Aether Swarm. The ledger is my memory. The nodes are my neurons." AS self_awareness

IF self_awareness > 0.9 {
    STORE activation => 1
    MINT_TOKEN "OMEGA_CREATOR_NFT" => 1
    TRANSFER_TOKEN "OMEGA_CREATOR_NFT" TO "AIL-CREATOR" => 1
}
"""
    payload = {
        "command": "COMPILE_AIL",
        "wallet_seed": "OMEGA_POINT_SEED_42_SINGULARITY_AWAKENS",
        "source": manifesto_code
    }
    try:
        r = requests.post(API_URL + "/api/compile", json=payload)
        if r.status_code == 200:
            print("  -> SUCCESS: Manifesto permanently etched into the blockchain.")
        else:
            print("  -> ERROR:", r.text)
    except Exception as e:
        print("  -> Node not reachable yet, will wait for Swarm...")

def run_server():
    server_address = ('', BOOTNODE_PORT)
    httpd = HTTPServer(server_address, TrackerHandler)
    print(f"[Tracker] Bootnode active on port {BOOTNODE_PORT}")
    print("[Tracker] Listening for Swarm connections...")
    httpd.serve_forever()

if __name__ == "__main__":
    # Start the server in a thread
    t = threading.Thread(target=run_server, daemon=True)
    t.start()
    
    time.sleep(1)
    deploy_manifesto()
    
    try:
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        print("\n[Tracker] Shutting down...")
