#include "legacy.h"
#include "image_info.h"
#include <algorithm>
#include <cctype>
#include <cstring>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <sstream>
#include <chrono>
#include <ctime>

namespace ldm {
void State::fail(const std::string& message) const {
    std::ostringstream o;
    o<<message<<" at "<<std::hex<<std::setfill('0')<<std::setw(4)<<uint16_t(cs-LoadSegment)
     <<":"<<std::setw(4)<<ip<<" ax="<<ax<<" bx="<<bx<<" cx="<<cx<<" dx="<<dx
     <<" ds="<<ds<<" es="<<es<<" ss:sp="<<ss<<":"<<sp;
    throw std::runtime_error(o.str());
}
void State::multiply(uint16_t value,int bits,bool sign) {
    bool overflow;
    if(bits==8) {
        int32_t r=sign?int32_t(int8_t(ax))*int8_t(value):uint32_t(uint8_t(ax))*uint8_t(value);
        ax=uint16_t(r); overflow=sign?r!=int8_t(r):(r&0xff00)!=0;
    } else {
        int64_t r=sign?int64_t(int16_t(ax))*int16_t(value):uint64_t(ax)*value;
        ax=uint16_t(r); dx=uint16_t(uint64_t(r)>>16);
        overflow=sign?r!=int16_t(r):(r&0xffff0000)!=0;
    }
    set_flag(CF,overflow);set_flag(OF,overflow);
}
void State::divide(uint16_t value,int bits,bool sign) {
    if(!value || (bits==8 && !uint8_t(value))) fail("Division by zero");
    int64_t numerator,denominator;
    if(bits==8) {numerator=sign?int16_t(ax):int32_t(ax);denominator=sign?int8_t(value):int32_t(uint8_t(value));}
    else {uint32_t n=(uint32_t(dx)<<16)|ax;numerator=sign?int64_t(int32_t(n)):int64_t(n);denominator=sign?int16_t(value):int32_t(value);}
    auto q=numerator/denominator,r=numerator%denominator;
    int64_t low=sign?-(1LL<<(bits-1)):0,high=sign?(1LL<<(bits-1))-1:(1LL<<bits)-1;
    if(q<low || q>high) fail("Division overflow");
    if(bits==8) ax=uint8_t(q)|(uint16_t(uint8_t(r))<<8);
    else {ax=uint16_t(q);dx=uint16_t(r);}
}
void State::string_op(const char* op,int bytes,int rep,uint16_t source_segment) {
    unsigned n=rep?cx:1;
    int delta=flag(DF)?-bytes:bytes;
    for(unsigned k=0;k<n;k++) {
        auto read=[&](uint16_t seg,uint16_t off){return bytes==1?u8(seg,off):u16(seg,off);};
        auto write=[&](uint16_t seg,uint16_t off,uint16_t v){if(bytes==1)w8(seg,off,v);else w16(seg,off,v);};
        if(!std::strcmp(op,"movs")){write(es,di,read(source_segment,si));si+=delta;di+=delta;}
        else if(!std::strcmp(op,"stos")){write(es,di,ax);di+=delta;}
        else if(!std::strcmp(op,"lods")){auto v=read(source_segment,si);ax=bytes==1?(ax&0xff00)|v:v;si+=delta;}
        else if(!std::strcmp(op,"scas")){alu(Op::Sub,ax,read(es,di),bytes*8);di+=delta;}
        else if(!std::strcmp(op,"cmps")){alu(Op::Sub,read(source_segment,si),read(es,di),bytes*8);si+=delta;di+=delta;}
        else fail("Unknown recovered string operation");
        if(rep)--cx;
        if((rep==2 && !flag(ZF)) || (rep==3 && flag(ZF))) break;
    }
}
std::string State::string_at(uint16_t seg,uint16_t off,char end) const {
    std::string s;
    for(unsigned i=0;i<65536;i++) {char c=u8(seg,uint16_t(off+i));if(c==end)return s;s+=c;}
    fail("Unterminated original string");return {};
}
static std::string lower(std::string s) {for(auto&c:s)c=char(std::tolower(static_cast<unsigned char>(c)));return s;}
std::filesystem::path State::file_path(const std::string& input,bool write) const {
    std::string name=input;std::replace(name.begin(),name.end(),'\\','/');
    if(name.size()>1 && name[1]==':')name=name.substr(2);
    while(!name.empty() && name[0]=='/')name.erase(0,1);
    std::filesystem::path relative(name);
    for(const auto& part:relative)if(part=="..")fail("Parent path in original file request");
    auto resolve=[&](std::filesystem::path root){
        for(const auto& part:relative) {
            auto next=root/part;
            if(!std::filesystem::exists(next) && std::filesystem::is_directory(root))
                for(auto& e:std::filesystem::directory_iterator(root))
                    if(lower(e.path().filename().string())==lower(part.string())){next=e.path();break;}
            root=next;
        }
        return root;
    };
    auto save=resolve(save_dir);
    if(write || std::filesystem::exists(save)) return save;
    return resolve(data_dir);
}
void State::load(const std::filesystem::path& image,const std::vector<uint32_t>& reloc,uint16_t initial_cs,uint16_t initial_ip,uint16_t initial_ss,uint16_t initial_sp) {
    std::ifstream f(image,std::ios::binary);
    if(!f)throw std::runtime_error("Cannot open recovered image");
    std::vector<uint8_t> b{std::istreambuf_iterator<char>(f),{}};
    if(b.size()>0x90000)throw std::runtime_error("Recovered image too large");
    uint64_t fingerprint=14695981039346656037ULL;
    for(auto byte:b)fingerprint=(fingerprint^byte)*1099511628211ULL;
    if(b.size()!=image_bytes || fingerprint!=image_fingerprint)throw std::runtime_error("Game data does not match this native build");
    std::copy(b.begin(),b.end(),memory.begin()+(LoadSegment<<4));
    for(auto p:reloc) {p+=LoadSegment<<4;if(p+1>=memory.size())throw std::runtime_error("Bad relocation");uint16_t v=memory[p]|(memory[p+1]<<8);v+=LoadSegment;memory[p]=v;memory[p+1]=v>>8;}
    cs=initial_cs+LoadSegment;ip=initial_ip;ss=initial_ss+LoadSegment;sp=initial_sp;ds=es=LoadSegment-16;
    w16(ds,0,0x20cd);w16(ds,2,0x9fff);w16(ds,0x2c,0x0f00);w8(ds,0x80,0);w8(ds,0x81,13);
    const char env[]="PATH=C:\\\0COMSPEC=C:\\COMMAND.COM\0\0\x01\0C:\\LDM.EXE\0";
    std::copy(std::begin(env),std::end(env),memory.begin()+0xf000);
    for(int v=0;v<256;v++){w16(0,uint16_t(v*4),uint16_t(v*4));w16(0,uint16_t(v*4+2),0xf000);}
    w16(0x40,0x10,0x21);w16(0x40,0x13,640);w8(0x40,0x49,3);w16(0x40,0x4a,80);w8(0x40,0x84,24);
    w8(0x40,0x87,0x60);w8(0x40,0x88,0x09);
    // IBM PS/2 BIOS Interface Technical Reference, April 1987, pp. 2-40..2-43.
    // Static video-function table: standard VGA modes and 200/350/400-line text.
    w8(0xf000,0x200,0xff);w8(0xf000,0x201,0xe0);w8(0xf000,0x202,0x0f);w8(0xf000,0x207,7);
    for(unsigned c=0;c<64;c++) {
        unsigned r=((c&4)?170:0)+((c&32)?85:0),g=((c&2)?170:0)+((c&16)?85:0),b=((c&1)?170:0)+((c&8)?85:0);
        palette[c]=0xff000000|(r<<16)|(g<<8)|b;
    }
    for(unsigned c=0;c<16;c++)attributes[c]=c<8?(c==6?20:c):(c+48);
}
void State::out(uint16_t port,uint16_t value,int bits) {
    if(port==0x388 || port==0x389)audio.write(port-0x388,uint8_t(value));
    if(port==0x43 && (value&0xc0)==0)pit_write_phase=0;
    if(port==0x43 && (value&0xc0)==0x80)speaker_write_phase=0;
    if(port==0x40){if(pit_write_phase++%2==0)pit_partial=value&255;else pit_divisor=pit_partial|((value&255)<<8);}
    if(port==0x42){if(speaker_write_phase++%2==0)speaker_partial=value&255;else speaker_divisor=speaker_partial|((value&255)<<8);}
    ports[port]=uint8_t(value);if(bits==16)ports[uint16_t(port+1)]=value>>8;
}
uint16_t State::in(uint16_t port,int bits) {
    if(port==0x388 || port==0x389)return audio.read(port-0x388);
    if(port==0x3da)ports[port]^=9;
    auto value=uint16_t(ports[port]);if(bits==16)value|=uint16_t(ports[uint16_t(port+1)])<<8;return value;
}
void State::timer_interrupt() {
    if(!flag(IF) || timer_active)return;
    auto handler_ip=u16(0,0x20),handler_cs=u16(0,0x22);
    if(handler_cs==0xf000){++ticks;w16(0x40,0x6c,ticks);w16(0x40,0x6e,ticks>>16);return;}
    const auto resume_cs=cs,resume_ip=ip,resume_sp=sp;
    push(flags);push(cs);push(ip);set_flag(IF,false);timer_active=true;
    cs=handler_cs;ip=handler_ip;
    auto start=boundaries;
    while(running && !(cs==resume_cs && ip==resume_ip && sp==resume_sp)) {
        if(boundaries-start>1000000)fail("Original timer handler did not return");
        if(cs==0xf000 && ip==0x20) {
            ++ticks;w16(0x40,0x6c,ticks);w16(0x40,0x6e,ticks>>16);
            ip=pop();cs=pop();flags=pop()|2;
        } else native_step(*this);
    }
    timer_active=false;
}
void State::interrupt(uint8_t number) {
    waiting=false;
    uint8_t ah=ax>>8,al=ax;
    auto ok=[&](){set_flag(CF,false);};
    auto error=[&](uint16_t n){set_flag(CF,true);ax=n;};
    if(number==0x20){running=false;return;}
    if(number==0x16) {
        if(ah==1 || ah==0x11){set_flag(ZF,keys.empty());if(!keys.empty())ax=keys.front();return;}
        if(ah==0 || ah==0x10){if(keys.empty()){waiting=true;return;}ax=keys.front();keys.erase(keys.begin());return;}
        if(ah==2){ax&=0xff00;return;}
    }
    if(number==0x33) {
        switch(ax) {
        case 0:ax=0xffff;bx=2;mouse_visibility=-1;return;
        case 1:++mouse_visibility;return;
        case 2:--mouse_visibility;return;
        case 7:case 8:case 0xa:case 0xf:return;
        case 9:mouse_hot_x=int16_t(bx);mouse_hot_y=int16_t(cx);for(unsigned i=0;i<32;i++)mouse_mask[i]=u16(es,uint16_t(dx+i*2));custom_cursor=true;return;
        case 3:bx=mouse_buttons;cx=uint16_t(mouse_x*2);dx=uint16_t(mouse_y);return;
        case 4:mouse_x=cx/2;mouse_y=dx;return;
        case 0xb:cx=dx=0;return;
        default:break;
        }
    }
    if(number==0x1a && ah==0){cx=ticks>>16;dx=ticks;ax&=0xff00;return;}
    if(number==0x10) {
        switch(ah) {
        case 0:video_mode=al&0x7f;w8(0x40,0x49,video_mode);w16(0x40,0x4a,(al==3)?80:40);std::cerr<<"video mode "<<std::hex<<video_mode<<std::dec<<"\n";return;
        case 1:case 2:case 5:return;
        case 3:cx=0x607;dx=0;return;
        case 0xf:ax=(uint16_t(u16(0x40,0x4a))<<8)|video_mode;bx&=255;return;
        case 0x1a:ax=(ax&0xff00)|0x1a;bx=8;return;
        case 0x1b:
            if(bx!=0){ax&=0xff00;return;}
            for(unsigned i=0;i<64;i++)w8(es,uint16_t(di+i),0);
            w16(es,di,0x200);w16(es,uint16_t(di+2),0xf000);
            w8(es,uint16_t(di+4),video_mode);w16(es,uint16_t(di+5),u16(0x40,0x4a));
            w16(es,uint16_t(di+7),video_mode==0x13?64000:0x4000);
            w16(es,uint16_t(di+0x1b),0x0607);w16(es,uint16_t(di+0x1e),0x3d4);
            w8(es,uint16_t(di+0x22),24);w16(es,uint16_t(di+0x23),8);
            w8(es,uint16_t(di+0x25),8);w16(es,uint16_t(di+0x27),video_mode==0x13?256:16);
            w8(es,uint16_t(di+0x29),1);w8(es,uint16_t(di+0x2d),1);w8(es,uint16_t(di+0x31),3);
            ax=(ax&0xff00)|0x1b;return;
        case 0x11:if(al==0x30){es=0xf000;bp=0xfa6e;cx=8;dx=24;return;}break;
        case 0x12:
            if(uint8_t(bx)==0x10){bx=3;cx=0;return;}
            if(uint8_t(bx)==0x33 && al==1){ax=(ax&0xff00)|0x12;return;} // Disable VGA grayscale summing; palette stays in color.
            break;
        case 0xef:return; // Hercules extension absent; leave DX=ffff for the original detection branch.
        case 0xe:std::cout<<char(al)<<std::flush;return;
        case 0xb:ports[0x3d9]=uint8_t(bx);return;
        case 0x10:
            if(al==0){if(uint8_t(bx)<16)attributes[uint8_t(bx)]=bx>>8;return;}
            if(al==1)return;
            if(al==2){for(int i=0;i<16;i++)attributes[i]=u8(es,uint16_t(dx+i));return;}
            if(al==0x10){palette[bx&255]=0xff000000|((uint32_t(dx>>8)*255/63)<<16)|((uint32_t(cx>>8)*255/63)<<8)|(uint32_t(cx&255)*255/63);return;}
            if(al==0x12){for(unsigned i=0;i<cx && bx+i<256;i++){auto p=uint16_t(dx+i*3);palette[bx+i]=0xff000000|((uint32_t(u8(es,p))*255/63)<<16)|((uint32_t(u8(es,p+1))*255/63)<<8)|(uint32_t(u8(es,p+2))*255/63);}return;}
            if(al==0x15){auto c=palette[bx&255];dx=uint16_t(((c>>16)&255)*63/255)<<8;cx=uint16_t(((c>>8)&255)*63/255)<<8|uint16_t((c&255)*63/255);return;}
            break;
        default:break;
        }
    }
    if(number==0x21) {
        switch(ah) {
        case 0x00:case 0x4c:running=false;return;
        case 0x07:case 0x08:
            if(keys.empty()){waiting=true;return;}
            ax=(ax&0xff00)|uint8_t(keys.front());keys.erase(keys.begin());return;
        case 0x09:std::cout<<string_at(ds,dx,'$')<<std::flush;return;
        case 0x0e:ax=(ax&0xff00)|3;return;
        case 0x19:ax=(ax&0xff00)|2;return;
        case 0x1a:ok();return;
        case 0x25:w16(0,al*4,dx);w16(0,al*4+2,ds);return;
        case 0x2a:case 0x2c: {
            auto now=std::chrono::system_clock::now();auto t=std::chrono::system_clock::to_time_t(now);auto local=*std::localtime(&t);
            if(ah==0x2a){cx=uint16_t(local.tm_year+1900);dx=uint16_t((local.tm_mon+1)<<8)|local.tm_mday;ax=(ax&0xff00)|local.tm_wday;}
            else {cx=uint16_t(local.tm_hour<<8)|local.tm_min;dx=uint16_t(local.tm_sec<<8)|uint16_t(std::chrono::duration_cast<std::chrono::milliseconds>(now.time_since_epoch()).count()%1000/10);}
            return;
        }
        case 0x30:ax=5;bx=cx=0;return;
        case 0x33:dx=0;return;
        case 0x35:bx=u16(0,al*4);es=u16(0,al*4+2);return;
        case 0x36:ax=1;bx=0x4000;cx=512;dx=0x8000;return;
        case 0x3c:case 0x3d: {
            auto name=string_at(ds,dx);bool write=ah==0x3c || (al&3)!=0;
            auto path=file_path(name,write);
            std::cerr<<"file "<<(write?"write ":"read ")<<name<<"\n";
            if(write)std::filesystem::create_directories(path.parent_path());
            if(write && ah==0x3d && !std::filesystem::exists(path)) {
                auto original=file_path(name,false);
                if(std::filesystem::is_regular_file(original))std::filesystem::copy_file(original,path);
            }
            #ifdef _WIN32
            auto file=_wfopen(path.c_str(),ah==0x3c?L"w+b":write?L"r+b":L"rb");
            #else
            auto file=std::fopen(path.c_str(),ah==0x3c?"w+b":write?"r+b":"rb");
            #endif
            if(!file){error(2);return;}
            int handle=5;while(files.count(handle))++handle;files[handle]=file;ax=handle;ok();return;
        }
        case 0x3e: {
            auto it=files.find(bx);if(it==files.end()){error(6);return;}
            std::fclose(it->second);files.erase(it);ok();return;
        }
        case 0x3f: {
            auto it=files.find(bx);if(it==files.end()){error(6);return;}
            std::vector<uint8_t> b(cx);auto n=std::fread(b.data(),1,cx,it->second);
            if(std::ferror(it->second)){error(5);return;}
            for(size_t i=0;i<n;i++)w8(ds,uint16_t(dx+i),b[i]);
            ax=n;ok();return;
        }
        case 0x40: {
            std::vector<uint8_t> b(cx);for(unsigned i=0;i<cx;i++)b[i]=u8(ds,uint16_t(dx+i));
            if(bx==1 || bx==2){std::cout.write(reinterpret_cast<char*>(b.data()),b.size());ax=cx;ok();return;}
            auto it=files.find(bx);if(it==files.end()){error(6);return;}
            auto n=std::fwrite(b.data(),1,b.size(),it->second);ax=n;ok();return;
        }
        case 0x42: {
            auto it=files.find(bx);if(it==files.end()){error(6);return;}
            int32_t off=int32_t((uint32_t(cx)<<16)|dx);int whence=al==0?SEEK_SET:al==1?SEEK_CUR:SEEK_END;
            if(al>2 || std::fseek(it->second,off,whence)){error(1);return;}
            long pos=std::ftell(it->second);ax=uint16_t(pos);dx=uint32_t(pos)>>16;ok();return;
        }
        case 0x43: {
            if(al!=0){error(1);return;}
            if(!std::filesystem::exists(file_path(string_at(ds,dx),false))){error(2);return;}cx=0;ok();return;
        }
        case 0x44:if(al==0){dx=bx<5?0x80d3:0;ok();return;}break;
        case 0x47:w8(ds,si,0);ax=0x100;ok();return;
        case 0x48:if(uint32_t(alloc_segment)+bx>0x9fff){bx=0x9fff-alloc_segment;error(8);return;}ax=alloc_segment;alloc_segment+=bx+1;ok();return;
        case 0x49:case 0x4a:ok();return;
        case 0x57:if(al==0){cx=0;dx=0x12e1;ok();return;}if(al==1){ok();return;}break;
        default:break;
        }
    }
    std::ostringstream message;message<<"Unimplemented platform service int "<<std::hex<<unsigned(number);
    fail(message.str());
}
}
