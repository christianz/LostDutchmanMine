#pragma once
#include <algorithm>
#include <chrono>
#include <deque>

namespace ldm {
struct MouseSample { int x=160,y=100,buttons=0; };

// Desktop events are edges; the original game polls button levels. Preserve
// each edge until a complete original mouse read has observed it. A short
// down/up pair must not disappear between simulation batches, and its click
// coordinates must survive a subsequent pointer move.
class MouseInput {
public:
    using Clock=std::chrono::steady_clock;
    void move(int x,int y) {
        current_.x=std::clamp(x,0,319);current_.y=std::clamp(y,0,199);
    }
    void buttons(int mask,Clock::time_point now=Clock::now()) {
        mask&=3;
        if(mask==current_.buttons)return;
        current_.buttons=mask;pending_.push_back({current_,now});
    }
    void poll(Clock::time_point now=Clock::now()) {
        // Ignore clicks made long before an interactive screen was ready
        // (e.g. during title/loading screens). Never replay those into a shop.
        while(!pending_.empty() && now-pending_.front().time>std::chrono::seconds(1))pending_.pop_front();
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
    struct Pending {MouseSample point;Clock::time_point time;};
    std::deque<Pending> pending_;
};
}
