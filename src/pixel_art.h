#pragma once
#include "display.h"
#include <algorithm>
#include <cstdlib>
#include <string>

namespace ldm::pixel {
inline void dot(Pixels& p,int x,int y,uint32_t c){if(x>=0 && x<320 && y>=0 && y<200)p[y*320+x]=c;}
inline void rect(Pixels& p,int x,int y,int w,int h,uint32_t c) {
    for(int yy=std::max(0,y);yy<std::min(200,y+h);++yy)
        for(int xx=std::max(0,x);xx<std::min(320,x+w);++xx)p[yy*320+xx]=c;
}
inline void line(Pixels& p,int x,int y,int xx,int yy,uint32_t c) {
    int dx=std::abs(xx-x),sx=x<xx?1:-1,dy=-std::abs(yy-y),sy=y<yy?1:-1,e=dx+dy;
    for(;;){dot(p,x,y,c);if(x==xx && y==yy)break;int e2=2*e;if(e2>=dy){e+=dy;x+=sx;}if(e2<=dx){e+=dx;y+=sy;}}
}
inline void ellipse(Pixels& p,int x,int y,int rx,int ry,uint32_t c,double tilt=0) {
    for(int yy=-ry;yy<=ry;yy++)for(int xx=-rx;xx<=rx;xx++)
        if(xx*xx*ry*ry+yy*yy*rx*rx<=rx*rx*ry*ry)dot(p,x+xx,y+yy+int(xx*tilt*.09),c);
}
// Small hand-authored 5x7 capitals: native game pixels, never a scaled desktop font.
inline void text(Pixels& p,int x,int y,const std::string& value,uint32_t c,bool centre=false) {
    static constexpr uint8_t glyphs[][7]={
        {14,17,19,21,25,17,14},{4,12,4,4,4,4,14},{14,17,1,2,4,8,31},
        {30,1,1,14,1,1,30},{2,6,10,18,31,2,2},{31,16,16,30,1,1,30},
        {14,16,16,30,17,17,14},{31,1,2,4,8,8,8},{14,17,17,14,17,17,14},
        {14,17,17,15,1,1,14},
        {14,17,17,31,17,17,17},{30,17,17,30,17,17,30},{14,17,16,16,16,17,14},
        {30,17,17,17,17,17,30},{31,16,16,30,16,16,31},{31,16,16,30,16,16,16},
        {14,17,16,23,17,17,15},{17,17,17,31,17,17,17},{14,4,4,4,4,4,14},
        {7,2,2,2,18,18,12},{17,18,20,24,20,18,17},{16,16,16,16,16,16,31},
        {17,27,21,21,17,17,17},{17,25,21,19,17,17,17},{14,17,17,17,17,17,14},
        {30,17,17,30,16,16,16},{14,17,17,17,21,18,13},{30,17,17,30,20,18,17},
        {15,16,16,14,1,1,30},{31,4,4,4,4,4,4},{17,17,17,17,17,17,14},
        {17,17,17,17,17,10,4},{17,17,17,21,21,21,10},{17,17,10,4,10,17,17},
        {17,17,10,4,4,4,4},{31,1,2,4,8,16,31}
    };
    if(centre)x-=int(value.size()*6-1)/2;
    for(char ch:value) {
        char u=ch>='a'&&ch<='z'?ch-32:ch;
        int n=u>='0'&&u<='9'?u-'0':u>='A'&&u<='Z'?u-'A'+10:-1;
        if(n>=0){for(int row=0;row<7;row++)for(int col=0;col<5;col++)if(glyphs[n][row]&(16>>col))dot(p,x+col,y+row,c);}
        else if(u=='.')rect(p,x+2,y+6,1,1,c);
        else if(u==':'){dot(p,x+2,y+2,c);dot(p,x+2,y+5,c);}
        else if(u=='!'){rect(p,x+2,y,1,4,c);dot(p,x+2,y+6,c);}
        else if(u=='/')line(p,x+4,y,x,y+6,c);
        else if(u=='-')rect(p,x,y+3,5,1,c);
        else if(u=='<'){line(p,x+3,y+1,x,y+3,c);line(p,x,y+3,x+3,y+5,c);}
        else if(u=='>'){line(p,x,y+1,x+3,y+3,c);line(p,x+3,y+3,x,y+5,c);}
        x+=6;
    }
}
}
