"""Deterministic generated-source -> seamless 512px runtime tiles and review derivatives.
Usage: python scripts/prepare-architecture-textures.py art-workshop/architecture-pilot/round3
No generation calls. Sources and request ledger remain immutable. Pillow and numpy required.
Normals/roughness are heuristic review assets; runtime uses procedural relief/roughness.
512px review masters become 256px runtime tiles to limit decoded GPU storage.
"""
from pathlib import Path
import sys,json,hashlib
from PIL import Image,ImageFilter,ImageDraw
import numpy as np
ROOT=Path(__file__).resolve().parents[1]
run=(ROOT/sys.argv[1]).resolve()
if not run.is_relative_to(ROOT/'art-workshop'):raise SystemExit('Run must be inside art-workshop')
config=json.loads((run/'comparison.json').read_text());ledger=json.loads((run/'ledger.json').read_text())
selected=run/'selected';selected.mkdir(exist_ok=True)
runtime=ROOT/'public/art/apartment-pilot';runtime.mkdir(exist_ok=True)
def periodic(a):
 h,w,c=a.shape;b=np.zeros_like(a);b[0]=a[-1]-a[0];b[-1]=a[0]-a[-1];b[:,0]+=a[:,-1]-a[:,0];b[:,-1]+=a[:,0]-a[:,-1]
 denom=2*np.cos(2*np.pi*np.arange(h)/h)[:,None]+2*np.cos(2*np.pi*np.arange(w)/w)[None,:]-4;denom[0,0]=1
 s=np.fft.fft2(b,axes=(0,1))/denom[:,:,None];s[0,0]=0
 return a-np.fft.ifft2(s,axes=(0,1)).real
manifest=[];sheet=Image.new('RGB',(5*256,2*290),'#e8e5dd');draw=ImageDraw.Draw(sheet)
for index,brief in enumerate(config['vehicles']):
 entry=next((r for r in ledger['requests'] if r['vehicle']==brief['id'] and r['status']=='complete'),None)
 if not entry:raise SystemExit('Missing completed source: '+brief['id'])
 name=brief['id'];source=run/entry['file'];im=Image.open(source).convert('RGB')
 if 'cropNormalized' in brief:
  crop=brief['cropNormalized'];im=im.crop(tuple(round(v*(im.width if i%2==0 else im.height)) for i,v in enumerate(crop)))
 im=im.resize((512,512),Image.Resampling.LANCZOS);a=np.asarray(im,dtype=float)/255
 low=np.asarray(im.filter(ImageFilter.GaussianBlur(28)),dtype=float)/255
 a=np.clip(periodic(a-.65*(low-low.mean(axis=(0,1)))),0,1)
 # Gently match the outer eight pixels after periodic low-frequency correction.
 for axis in [0,1]:
  moved=np.moveaxis(a,axis,0)
  for edge in range(8):
   left=moved[edge].copy();right=moved[-1-edge].copy();weight=(1-edge/8)*.5
   moved[edge]=left*(1-weight)+right*weight;moved[-1-edge]=right*(1-weight)+left*weight
 tile=Image.fromarray(np.uint8(a*255+.5));tile.save(selected/(name+'.png'))
 runtime_tile=tile.resize((256,256),Image.Resampling.LANCZOS)
 pixels=np.asarray(runtime_tile).copy();pixels[:,0]=pixels[:,-1]=((pixels[:,0].astype(float)+pixels[:,-1])/2).astype('uint8');pixels[0]=pixels[-1]=((pixels[0].astype(float)+pixels[-1])/2).astype('uint8')
 Image.fromarray(pixels).save(runtime/(name+'.png'))
 lum=a@np.array([.2126,.7152,.0722]);h=lum-lum.mean();dx=(np.roll(h,-1,1)-np.roll(h,1,1))*.35;dy=(np.roll(h,-1,0)-np.roll(h,1,0))*.35
 n=np.stack((-dx,dy,np.ones_like(h)),axis=2);n/=np.linalg.norm(n,axis=2,keepdims=True)
 Image.fromarray(np.uint8((n*.5+.5)*255+.5)).save(selected/(name+'-normal.png'))
 rough=.45 if 'metal' in name else .74 if 'ceramic' in name or 'porcelain' in name else .85
 Image.fromarray(np.uint8(np.clip(rough+h*.12,.2,.95)*255+.5)).save(selected/(name+'-roughness.png'))
 repeat=Image.new('RGB',(1024,1024))
 for x in range(2):
  for y in range(2):repeat.paste(tile,(x*512,y*512))
 repeat.save(selected/(name+'-repeat.jpg'))
 x=index%5*256;y=index//5*290;sheet.paste(tile.resize((256,256)),(x,y));draw.text((x+6,y+263),name,fill='#203331')
 seam=float((np.abs(a[:,0]-a[:,-1]).mean()+np.abs(a[0]-a[-1]).mean())/2*255)
 interior=float((np.abs(a[:,1:]-a[:,:-1]).mean()+np.abs(a[1:]-a[:-1]).mean())/2*255)
 payload=(runtime/(name+'.png')).read_bytes()
 manifest.append({'id':name,'source':str(source.relative_to(ROOT)),'cropNormalized':brief.get('cropNormalized'),'sourceSha256':hashlib.sha256(source.read_bytes()).hexdigest(),'path':str((runtime/(name+'.png')).relative_to(ROOT)),'sha256':hashlib.sha256(payload).hexdigest(),'bytes':len(payload),'width':256,'height':256,'reviewMasterSide':512,'tileMeters':.4,'boundaryMeanDifference':round(seam,3),'interiorMeanDifference':round(interior,3),'seamRatio':round(seam/max(interior,.001),3),'normalAndRoughness':'heuristic derivatives, not runtime maps','model':entry['model'],'costUSD':entry['costUSD'],'prompt':entry['prompt'],'status':'prepared-for-visual-review'})
(run/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n');sheet.save(run/'material-sheet.jpg',quality=92)
print(json.dumps({'textures':len(manifest),'runtimeBytes':sum(r['bytes'] for r in manifest),'maxSeamDifference':max(r['boundaryMeanDifference'] for r in manifest),'costUSD':sum(r['costUSD'] for r in manifest)},indent=2))
