#pragma once
#include <algorithm>
#include <cstdint>
#include <deque>

namespace ldm {
struct MouseSample { int x=160,y=100,buttons=0; };

// Desktop events are edges; the original game polls button levels. Preserve
// each edge until a complete original mouse read has observed it. A short
// down/up pair must not disappear between simulation batches, and its click
// coordinates must survive a subsequent pointer move.
class MouseInput {
public:
    // Emulated milliseconds, set by the session before each simulation quantum.
    void set_time(uint64_t now_ms) {now_ms_=now_ms;}
    void move(int x,int y) {
        current_.x=std::clamp(x,0,319);current_.y=std::clamp(y,0,199);
    }
    void buttons(int mask) {
        mask&=3;
        if(mask==current_.buttons)return;
        current_.buttons=mask;pending_.push_back({current_,now_ms_});
    }
    void poll() {
        // Ignore clicks made long before an interactive screen was ready
        // (e.g. during title/loading screens). Never replay those into a shop.
        while(!pending_.empty() && now_ms_>pending_.front().time_ms+1000)pending_.pop_front();
        if(pending_.empty())sample_=current_;
        else {sample_=pending_.front().point;pending_.pop_front();}
    }
    void clear() {
        current_.buttons=0;discard_pending();
    }
    void discard_pending() {
        pending_.clear();sample_=current_;
    }
    void warp(int x,int y) {
        move(x,y);sample_.x=current_.x;sample_.y=current_.y;
    }
    const MouseSample& current() const {return current_;}
    const MouseSample& sample() const {return sample_;}
private:
    MouseSample current_,sample_;
    struct Pending {MouseSample point;uint64_t time_ms;};
    std::deque<Pending> pending_;
    uint64_t now_ms_=0;
};
}
