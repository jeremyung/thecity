"""Local half of the vehicle texture pilot: HunyuanImage-2.1 through a headless
ComfyUI server (http://127.0.0.1:8188). Same prompts as the hosted run.

usage: generate-vehicle-pilot-local.py <unet file> <steps> <cfg> <sampler> <tag> [vehicle ...]
"""
import json, os, sys, time, urllib.request, pathlib, shutil

ROOT = pathlib.Path(__file__).resolve().parent.parent
CONFIG = json.loads((ROOT / os.environ.get('VEHICLE_CONFIG', 'art-workshop/vehicle-pilot.json')).read_text())
# Livery configs reuse the pilot's sampler/resolution settings.
CONFIG.setdefault('local', json.loads((ROOT / 'art-workshop/vehicle-pilot.json').read_text())['local'])
OUT = ROOT / 'art-workshop/vehicle-pilot'
COMFY = 'http://127.0.0.1:8188'
COMFY_OUTPUT = pathlib.Path('/home/jung/Development/ComfyUI/output')
NEGATIVE = os.environ.get('NEGATIVE_EXTRA', '') + os.environ.get('NEGATIVE_BASE', 'perspective, three-quarter view, logo, badge, emblem, text, letters, licence plate characters, watermark, cast shadow, reflections, people, background scenery, cropped vehicle')

def call(path, body=None):
    req = urllib.request.Request(COMFY + path, data=json.dumps(body).encode() if body else None, headers={'Content-Type': 'application/json'})
    with urllib.request.urlopen(req) as r: return json.load(r)

def workflow(prompt, unet, steps, cfg, sampler, seed, prefix):
    local = CONFIG['local']
    return {
        '1': {'class_type': 'UNETLoader', 'inputs': {'unet_name': unet, 'weight_dtype': 'default'}},
        '2': {'class_type': 'DualCLIPLoader', 'inputs': {'clip_name1': 'qwen_2.5_vl_7b.safetensors', 'clip_name2': 'byt5_small_glyphxl_fp16.safetensors', 'type': 'hunyuan_image', 'device': 'default'}},
        '3': {'class_type': 'VAELoader', 'inputs': {'vae_name': 'hunyuan_image_2.1_vae_fp16.safetensors'}},
        '4': {'class_type': 'CLIPTextEncode', 'inputs': {'text': prompt, 'clip': ['2', 0]}},
        '5': {'class_type': 'CLIPTextEncode', 'inputs': {'text': NEGATIVE, 'clip': ['2', 0]}},
        '6': {'class_type': 'EmptyHunyuanImageLatent', 'inputs': {'width': local['width'], 'height': local['height'], 'batch_size': 1}},
        '7': {'class_type': 'KSampler', 'inputs': {'model': ['1', 0], 'positive': ['4', 0], 'negative': ['5', 0], 'latent_image': ['6', 0], 'seed': seed, 'steps': steps, 'cfg': cfg, 'sampler_name': sampler, 'scheduler': 'simple', 'denoise': 1}},
        '8': {'class_type': 'VAEDecode', 'inputs': {'samples': ['7', 0], 'vae': ['3', 0]}},
        '9': {'class_type': 'SaveImage', 'inputs': {'images': ['8', 0], 'filename_prefix': prefix}},
    }

def main():
    unet, steps, cfg, sampler, tag = sys.argv[1], int(sys.argv[2]), float(sys.argv[3]), sys.argv[4], sys.argv[5]
    wanted = set(sys.argv[6:])
    log = OUT / 'local-ledger.json'
    ledger = json.loads(log.read_text()) if log.exists() else []
    for vehicle in CONFIG['vehicles']:
        if wanted and vehicle['id'] not in wanted: continue
        name = f"{tag}--{vehicle['id']}"
        prompt = f"{CONFIG['commonPrompt']} {vehicle['prompt']} {os.environ.get('PROMPT_SUFFIX', '')}".strip()
        started = time.time()
        pid = call('/prompt', {'prompt': workflow(prompt, unet, steps, cfg, sampler, int(os.environ.get('SEED', CONFIG['local']['seed'])), name)})['prompt_id']
        while True:
            hist = call(f'/history/{pid}')
            if pid in hist: break
            time.sleep(1)
        entry = hist[pid]
        if entry['status']['status_str'] != 'success':
            print(name, 'FAILED', json.dumps(entry['status'])[:800]); sys.exit(1)
        image = next(i for node in entry['outputs'].values() for i in node.get('images', []))
        shutil.copy(COMFY_OUTPUT / image['subfolder'] / image['filename'], OUT / f'{name}.png')
        seconds = round(time.time() - started, 1)
        ledger.append({'id': name, 'unet': unet, 'steps': steps, 'cfg': cfg, 'sampler': sampler, 'seconds': seconds, 'costUSD': 0})
        log.write_text(json.dumps(ledger, indent=2) + '\n')
        print(name, seconds, 's')

main()
