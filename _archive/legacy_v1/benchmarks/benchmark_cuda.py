#!/usr/bin/env python3
"""
PyTorch/CUDA Baseline Benchmark
Matches the HLX transformer architecture exactly for fair comparison
"""

import torch
import torch.nn as nn
import torch.optim as optim
import time
import json
from pathlib import Path

class TinyTransformer(nn.Module):
    """Matches HLX architecture: 4 layers, d_model=256, ffn_dim=1024"""
    def __init__(self, vocab_size=128, d_model=256, ffn_dim=1024, num_layers=4, max_seq_len=16):
        super().__init__()
        self.d_model = d_model

        # Embeddings
        self.token_emb = nn.Embedding(vocab_size, d_model)
        self.pos_emb = nn.Embedding(max_seq_len, d_model)

        # Transformer layers
        self.layers = nn.ModuleList([
            nn.ModuleDict({
                'ln1': nn.LayerNorm(d_model),
                'v_proj': nn.Linear(d_model, d_model, bias=False),
                'o_proj': nn.Linear(d_model, d_model, bias=False),
                'ln2': nn.LayerNorm(d_model),
                'ffn_w1': nn.Linear(d_model, ffn_dim, bias=True),
                'ffn_w2': nn.Linear(ffn_dim, d_model, bias=True),
            })
            for _ in range(num_layers)
        ])

        # Output
        self.final_ln = nn.LayerNorm(d_model)
        self.output_proj = nn.Linear(d_model, vocab_size, bias=False)

    def forward(self, input_ids):
        batch_size, seq_len = input_ids.shape

        # Embeddings
        pos = torch.arange(seq_len, device=input_ids.device).unsqueeze(0)
        x = self.token_emb(input_ids) + self.pos_emb(pos)

        # Transformer layers
        for layer in self.layers:
            # Simplified attention (just V projection like HLX)
            residual = x
            x = layer['ln1'](x)
            v = layer['v_proj'](x)
            x = layer['o_proj'](v)
            x = x + residual

            # FFN
            residual = x
            x = layer['ln2'](x)
            x = layer['ffn_w1'](x)
            x = torch.nn.functional.gelu(x)
            x = layer['ffn_w2'](x)
            x = x + residual

        # Output
        x = self.final_ln(x)
        logits = self.output_proj(x)

        return logits

def dump_grad_stats(name, tensor):
    """Compute and print gradient statistics for a tensor"""
    if tensor is None or tensor.grad is None:
        print(f"  [{name}] NO GRADIENT")
        return

    grad = tensor.grad.detach()
    mean = grad.mean().item()
    std = grad.std().item()
    min_val = grad.min().item()
    max_val = grad.max().item()
    abs_max = grad.abs().max().item()
    zeros = (grad == 0).sum().item()
    nans = torch.isnan(grad).sum().item()
    infs = torch.isinf(grad).sum().item()

    print(f"  [{name}] shape={list(grad.shape)} mean={mean:.6f} std={std:.6f} min={min_val:.6f} max={max_val:.6f} abs_max={abs_max:.6f} zeros={zeros} nan={nans} inf={infs}")

def load_corpus(path="corpus.jsonl"):
    """Load training corpus"""
    examples = []
    with open(path) as f:
        for line in f:
            if line.strip():
                import json
                ex = json.loads(line)
                examples.append(ex)
    return examples

def tokenize_simple(text):
    """Simple char-level tokenization matching HLX"""
    return [ord(c) if ord(c) < 128 else 127 for c in text]

def create_batches(examples, batch_size=4, max_seq_len=16):
    """Create training batches"""
    batches = []
    for i in range(0, len(examples), batch_size):
        batch_examples = examples[i:i+batch_size]

        input_ids = []
        target_ids = []

        for ex in batch_examples:
            text = f"{ex['input']} -> {ex['output']}"
            tokens = tokenize_simple(text)[:max_seq_len]

            # Pad
            while len(tokens) < max_seq_len:
                tokens.append(0)

            input_ids.append(tokens[:-1] + [0])
            target_ids.append(tokens[1:] + [0])

        batches.append({
            'input_ids': torch.tensor(input_ids, dtype=torch.long),
            'target_ids': torch.tensor(target_ids, dtype=torch.long)
        })

    return batches

def train_cuda_baseline(num_epochs=20, device='cuda'):
    """Train PyTorch model on CUDA for baseline comparison"""
    print("╔══════════════════════════════════════════════════════╗")
    print("║     PyTorch CUDA Baseline Training                   ║")
    print("╚══════════════════════════════════════════════════════╝\n")

    # Model setup
    model = TinyTransformer().to(device)
    optimizer = optim.Adam(model.parameters(), lr=3e-4, betas=(0.9, 0.999))

    # Count parameters
    total_params = sum(p.numel() for p in model.parameters())
    print(f"Model: 4 layers, d_model=256, ffn_dim=1024")
    print(f"  Parameters: {total_params/1e6:.2f}M\n")

    # Load data
    examples = load_corpus()
    print(f"  Loaded {len(examples)} examples")

    batches = create_batches(examples, batch_size=4, max_seq_len=16)
    print(f"  Created {len(batches)} batches\n")

    print("🚀 Starting training...\n")

    best_loss = float('inf')
    results = []

    for epoch in range(1, num_epochs + 1):
        epoch_start = time.time()
        epoch_loss = 0.0
        total_tokens = 0

        for batch_idx, batch in enumerate(batches):
            input_ids = batch['input_ids'].to(device)
            target_ids = batch['target_ids'].to(device)

            # Forward
            logits = model(input_ids)

            # Loss
            loss = nn.functional.cross_entropy(
                logits.reshape(-1, 128),
                target_ids.reshape(-1),
                ignore_index=0
            )

            # Backward
            optimizer.zero_grad()
            loss.backward()

            # DEBUG: Dump gradient statistics for first batch of first epoch
            if epoch == 1 and batch_idx == 0:
                print("\n=== CUDA GRADIENT STATISTICS (Epoch 1, Batch 0) ===")

                # Embedding gradients
                dump_grad_stats("token_emb", model.token_emb.weight)
                dump_grad_stats("pos_emb", model.pos_emb.weight)

                # Layer 0 gradients (for comparison with HLX)
                layer = model.layers[0]
                dump_grad_stats("L0_ln1_gamma", layer['ln1'].weight)
                dump_grad_stats("L0_ln1_beta", layer['ln1'].bias)
                dump_grad_stats("L0_v_proj", layer['v_proj'].weight)
                dump_grad_stats("L0_o_proj", layer['o_proj'].weight)
                dump_grad_stats("L0_ln2_gamma", layer['ln2'].weight)
                dump_grad_stats("L0_ln2_beta", layer['ln2'].bias)
                dump_grad_stats("L0_ffn_w1", layer['ffn_w1'].weight)
                dump_grad_stats("L0_ffn_b1", layer['ffn_w1'].bias)
                dump_grad_stats("L0_ffn_w2", layer['ffn_w2'].weight)
                dump_grad_stats("L0_ffn_b2", layer['ffn_w2'].bias)

                # Final LayerNorm and output
                dump_grad_stats("final_ln_gamma", model.final_ln.weight)
                dump_grad_stats("final_ln_beta", model.final_ln.bias)
                dump_grad_stats("output_proj", model.output_proj.weight)

                print("=== END CUDA GRADIENT STATISTICS ===\n")

            optimizer.step()

            epoch_loss += loss.item() * input_ids.numel()
            total_tokens += input_ids.numel()

        epoch_time = time.time() - epoch_start
        avg_loss = epoch_loss / total_tokens
        tok_per_sec = int(total_tokens / epoch_time)

        is_best = avg_loss < best_loss
        if is_best:
            best_loss = avg_loss

        star = "★" if is_best else " "
        lr = optimizer.param_groups[0]['lr']

        print(f"Epoch {epoch:3d}/{num_epochs}: loss={avg_loss:.4f} lr={lr:.2e} "
              f"time={int(epoch_time*1000):4d}ms tok/s={tok_per_sec:5d} {star}")

        results.append({
            'epoch': epoch,
            'loss': avg_loss,
            'time_ms': int(epoch_time * 1000),
            'tokens_per_sec': tok_per_sec,
            'is_best': is_best
        })

    print(f"\n🎉 Training complete!")
    print(f"  Best loss: {best_loss:.4f}")

    return results, best_loss

if __name__ == '__main__':
    # Check CUDA availability
    if not torch.cuda.is_available():
        print("WARNING: CUDA not available, using CPU")
        device = 'cpu'
    else:
        device = 'cuda'
        print(f"Using GPU: {torch.cuda.get_device_name(0)}\n")

    results, best_loss = train_cuda_baseline(num_epochs=100, device=device)

    # Save results
    output_dir = Path("benchmarks/results")
    output_dir.mkdir(parents=True, exist_ok=True)

    with open(output_dir / "cuda_results.json", 'w') as f:
        json.dump({
            'device': device,
            'best_loss': best_loss,
            'results': results
        }, f, indent=2)

    print(f"\nResults saved to benchmarks/results/cuda_results.json")
