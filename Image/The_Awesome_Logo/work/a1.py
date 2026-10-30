import cv2, numpy as np
im=cv2.imread('work/dark_panel.png')
x0,y0,x1,y1=160,85,450,310
c=im[y0:y1,x0:x1]; S=4
big=cv2.resize(c,None,fx=S,fy=S,interpolation=cv2.INTER_CUBIC)
g=cv2.cvtColor(big,cv2.COLOR_BGR2GRAY)
g=cv2.GaussianBlur(g,(0,0),1.5)
print('bg lum',np.percentile(g[:20],50))
for t in (30,45,60):
    m=(g>t).astype(np.uint8)*255
    cs,h=cv2.findContours(m,cv2.RETR_CCOMP,cv2.CHAIN_APPROX_NONE)
    cs2=[(cv2.contourArea(k),i) for i,k in enumerate(cs) if cv2.contourArea(k)>500]
    print('t',t,[(int(a),h[0][i][3]) for a,i in cs2])
m=(g>45).astype(np.uint8)*255
cs,h=cv2.findContours(m,cv2.RETR_CCOMP,cv2.CHAIN_APPROX_NONE)
for i,k in enumerate(cs):
    if cv2.contourArea(k)>500:
        ap=cv2.approxPolyDP(k,6,True)[:,0]
        print('contour',i,'parent',h[0][i][3],'bbox',cv2.boundingRect(k));print(ap.tolist())
cv2.imwrite('work/sym_big.png',big)
