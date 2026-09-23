#include "legacy.h"
#include <iostream>
#include <memory>
#include <stdexcept>

int main() {
    auto p=std::make_unique<ldm::State>();auto&s=*p;uint64_t checked=0;
    try {
        for(int bits:{8,16}) {
            uint32_t mask=(1u<<bits)-1,sign=1u<<(bits-1);
            for(uint32_t a=0;a<=mask;a++)for(uint32_t b:{0u,1u,sign-1,sign,mask})for(int carry:{0,1}) {
                for(bool sub:{false,true}) {
                    s.flags=2|(carry?ldm::CF:0)|ldm::DF;
                    auto result=s.alu(sub?ldm::Op::Sbb:ldm::Op::Adc,a,b,bits);
                    int64_t exact=sub?int64_t(a)-b-carry:int64_t(a)+b+carry;
                    auto signed_a=(a&sign)?int64_t(a)-mask-1:int64_t(a);
                    auto signed_b=(b&sign)?int64_t(b)-mask-1:int64_t(b);
                    auto signed_result=sub?signed_a-signed_b-carry:signed_a+signed_b+carry;
                    bool overflow=signed_result< -int64_t(sign) || signed_result>=sign;
                    if(result!=(exact&mask) || s.flag(ldm::CF)!=(exact<0 || exact>mask) || s.flag(ldm::OF)!=overflow ||
                       s.flag(ldm::ZF)!=(result==0) || s.flag(ldm::SF)!=bool(result&sign) || !s.flag(ldm::DF))
                        throw std::runtime_error("Arithmetic boundary mismatch");
                    unsigned ones=0;for(unsigned i=0;i<8;i++)ones+=(result>>i)&1;
                    if(s.flag(ldm::PF)!=!(ones&1))throw std::runtime_error("Parity mismatch");
                    ++checked;
                }
            }
        }
        for(int carry:{0,1}) {
            s.flags=carry?ldm::CF:0;s.alu(ldm::Op::Inc,0xffff,1,16);
            if(s.flag(ldm::CF)!=bool(carry))throw std::runtime_error("INC changed carry");
            s.alu(ldm::Op::Dec,0,1,16);if(s.flag(ldm::CF)!=bool(carry))throw std::runtime_error("DEC changed carry");
        }
        s.ax=uint16_t(-32768);s.multiply(uint16_t(-1),16,true);
        if(s.dx!=0 || s.ax!=32768 || !s.flag(ldm::OF))throw std::runtime_error("Signed multiply mismatch");
        s.dx=0xffff;s.ax=uint16_t(-1000);s.divide(33,16,true);
        if(int16_t(s.ax)!=-30 || int16_t(s.dx)!=-10)throw std::runtime_error("Signed divide mismatch");
        std::cout<<"PASS: "<<checked<<" independently computed arithmetic/flag cases plus carry and signed arithmetic boundaries\n";
    }catch(const std::exception&e){std::cerr<<e.what()<<"\n";return 1;}
}
