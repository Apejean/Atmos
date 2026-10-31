import cv2, numpy as np, sys
big=cv2.imread('work/sym_big.png')
# brighten dark areas for visibility
f=(255*((big/255.0)**0.6)).astype(np.uint8)
def crop(name,xa,ya,xb,yb,z=1.0):
    c=f[ya:yb,xa:xb].copy()
    c=cv2.resize(c,None,fx=z,fy=z,interpolation=cv2.INTER_CUBIC)
    for x in range((xa//25+1)*25,xb,25):
        X=int((x-xa)*z); cv2.line(c,(X,0),(X,c.shape[0]),(0,200,255) if x%100==0 else (80,80,160),1)
        if x%50==0: cv2.putText(c,str(x),(X+2,12),0,0.4,(0,255,0),1)
    for y in range((ya//25+1)*25,yb,25):
        Y=int((y-ya)*z); cv2.line(c,(0,Y),(c.shape[1],Y),(0,200,255) if y%100==0 else (80,80,160),1)
        if y%50==0: cv2.putText(c,str(y),(2,Y-2),0,0.4,(0,255,0),1)
    cv2.imwrite(name,c)
crop('work/g_top.png',480,30,800,330,3)
crop('work/g_mid.png',400,440,1060,720,1.7)
crop('work/g_bot.png',640,640,1120,880,2.3)
crop('work/g_botl.png',30,500,500,880,2.4)
