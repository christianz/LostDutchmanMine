#include "legacy.h"
#include "image_info.h"
#include <algorithm>
#include <array>
#include <iostream>
#include <memory>
#include <stdexcept>

// Exercise the actual original qsort call, including its far comparator callback
// at 0106:0000. This is the callback missing from the first native poker build.
int main() {
    try {
        std::array<uint16_t,5> order{0,1,2,3,4};
        const std::array<uint16_t,5> ranks{2,7,14,10,3};
        unsigned cases=0;
        do {
            auto s=std::make_unique<ldm::State>();
            s->load("recovered/load-image.bin",image_relocations,entry_cs,entry_ip,stack_ss,stack_sp);
            s->ds=s->ss=0x82bd;s->sp=0x8000;
            for(unsigned i=0;i<5;i++) {
                s->w16(s->ds,0x6000+4*i,ranks[order[i]]);
                s->w16(s->ds,0x6002+4*i,order[i]);
            }
            s->push(ldm::LoadSegment+0x0106);s->push(0); // comparator
            s->push(4);s->push(5);s->push(0x6000); // record size, count, base
            s->push(0xffff);s->push(0xfffe); // far return sentinel
            s->cs=ldm::LoadSegment+0x13b4;s->ip=0x227c;
            while(!(s->cs==0xffff && s->ip==0xfffe)) {
                if(s->boundaries>100000)throw std::runtime_error("Poker sort did not return");
                ldm::native_step(*s);
            }
            if(s->sp!=0x8000-10)throw std::runtime_error("Poker callback corrupted stack");
            uint16_t previous=0xffff;unsigned seen=0;
            for(unsigned i=0;i<5;i++) {
                auto rank=s->u16(s->ds,0x6000+4*i),id=s->u16(s->ds,0x6002+4*i);
                if(rank>previous || id>=5 || rank!=ranks[id] || (seen&(1u<<id)))
                    throw std::runtime_error("Poker cards sorted incorrectly");
                previous=rank;seen|=1u<<id;
            }
            ++cases;
        }while(std::next_permutation(order.begin(),order.end()));
        std::cout<<"PASS: all "<<cases<<" poker-hand permutations through the original sort and callback\n";
    }catch(const std::exception& e){std::cerr<<e.what()<<"\n";return 1;}
}
