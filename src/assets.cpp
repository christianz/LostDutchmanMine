#include "assets.h"
#include <stdexcept>
#include <cstddef>
#include <string>
namespace ldm {
std::vector<uint8_t> decode_asset(const std::vector<uint8_t>& p) {
    auto fail=[](){throw std::runtime_error("Invalid LDM asset stream");};
    if(p.size()<6 || p[0]!=1 || p[1]!=0x9d)fail();
    size_t packed=p[2]|(size_t(p[3])<<8),expected=p[4]|(size_t(p[5])<<8);
    if(packed+6!=p.size())fail();
    std::vector<uint8_t> out;out.reserve(expected);
    std::vector<size_t> dictionary;
    size_t bit=48,tokens=0;unsigned width=9;
    while(bit+width<=p.size()*8) {
        if(tokens%2==0)dictionary.push_back(out.size());
        unsigned code=0;
        while(true) {
            if(width>16)fail();
            if(bit+width>p.size()*8) {
                if(out.size()<expected)fail();
                return out;
            }
            code=0;
            for(unsigned b=0;b<width;b++,bit++)code=(code<<1)|((p[bit/8]>>(7-bit%8))&1);
            if(code!=256)break;
            ++width;
        }
        if(code<256)out.push_back(uint8_t(code));
        else {
            size_t n=code-257;
            if(n+1>=dictionary.size())fail();
            size_t start=dictionary[n],end=dictionary[n+1];
            if(end>out.size() || start>=end || end-start>65535-out.size())fail();
            // Use indices: insertion may reallocate, invalidating vector iterators.
            for(size_t i=start;i<end;i++)out.push_back(out[i]);
        }
        if(out.size()>65535)fail();
        ++tokens;
    }
    // The original decoder ignores the declared size. Some shipped streams
    // decode zero padding beyond it (e.g. POKR_EGA is 64002, header says 64000).
    if(out.size()<expected)throw std::runtime_error("LDM asset shorter than its declared size");
    return out;
}
}
