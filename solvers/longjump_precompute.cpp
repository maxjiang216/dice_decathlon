// g++ -O3 -std=c++20 longjump_precompute_simple.cpp -lsqlite3 -o longjump_precompute_simple
// Usage: ./longjump_precompute_simple longjump_policy_simple.db
//
// Long Jump (Knizia's Decathlon) — Optimizes single-attempt EV and stores
// policy as just the number of dice to freeze at each step (freeze smallest
// first in run-up, largest first in jump).
//
// Tables:
//   lj_post_simple(phase,sum_frozen,n1..n6,freeze_count)
//   lj_meta(key,value)

#include <bits/stdc++.h>
#include <sqlite3.h>
using namespace std;

enum Phase : int { RUNUP_POST=1, JUMP_POST=3 };

struct Counts {
    int c[7];
    Counts(){ memset(c,0,sizeof(c)); }
    int total() const { int t=0; for(int i=1;i<=6;i++) t+=c[i]; return t; }
    int sum() const { int s=0; for(int i=1;i<=6;i++) s+=i*c[i]; return s; }
    bool operator==(const Counts &o) const {
        for(int i=1;i<=6;i++) if(c[i]!=o.c[i]) return false;
        return true;
    }
};
struct CountsHash {
    size_t operator()(const Counts &x) const noexcept {
        size_t h=0;
        for(int i=1;i<=6;i++) h = h*1315423911u + x.c[i];
        return h;
    }
};

static vector<pair<Counts,double>> outcomes_cache[6];

static void build_outcomes(){
    for(int n=1;n<=5;n++){
        map<array<int,6>, double> m;
        int total = pow(6,n);
        function<void(int,array<int,6>&)> rec = [&](int left, array<int,6>& a){
            if(left==0){ m[a] += 1.0/total; return; }
            for(int face=1; face<=6; ++face){
                a[face-1]++; rec(left-1,a); a[face-1]--;
            }
        };
        array<int,6> a{}; rec(n,a);
        vector<pair<Counts,double>> vec;
        for(auto &kv: m){
            Counts cnt; for(int i=1;i<=6;i++) cnt.c[i]=kv.first[i-1];
            vec.push_back({cnt, kv.second});
        }
        outcomes_cache[n] = move(vec);
    }
}

struct RunupKey { int sum_frozen; Counts cnt; };
struct JumpKey   { Counts cnt; };

struct RKHash { size_t operator()(const RunupKey& k) const noexcept {
    CountsHash h; size_t x=1469598103934665603ull;
    x ^= (size_t)k.sum_frozen + 0x9e3779b97f4a7c15ull + (x<<6)+(x>>2);
    x ^= h(k.cnt) + 0x9e3779b97f4a7c15ull + (x<<6)+(x>>2);
    return x;
}};
struct RKEq { bool operator()(const RunupKey& a,const RunupKey& b) const noexcept {
    return a.sum_frozen==b.sum_frozen && a.cnt==b.cnt;
}};
struct JKHash { size_t operator()(const JumpKey& k) const noexcept {
    CountsHash h; return h(k.cnt);
}};
struct JKEq { bool operator()(const JumpKey& a,const JumpKey& b) const noexcept {
    return a.cnt==b.cnt;
}};

static unordered_map<RunupKey,int,RKHash,RKEq> best_runup_freeze;
static unordered_map<JumpKey,int,JKHash,JKEq>   best_jump_freeze;

struct Moments { double ev=0, ev2=0; };

static map<pair<int,int>, Moments> memo_runup;
static map<int, Moments> memo_jump;

static Moments solve_jump_pre(int n_rem){
    if(n_rem==0) return {0.0,0.0};
    if(memo_jump.count(n_rem)) return memo_jump[n_rem];
    double ev=0, ev2=0;
    for(auto& pr: outcomes_cache[n_rem]){
        const Counts& cnt=pr.first; double p=pr.second;
        double best_ev=-1e100, best_ev2=0; int best_fc=1;
        for(int freeze_count=1; freeze_count<=n_rem; ++freeze_count){
            // freeze_count largest dice
            int frozen_sum=0;
            int needed=freeze_count;
            for(int face=6; face>=1 && needed>0; --face){
                int take=min(cnt.c[face], needed);
                frozen_sum += face*take;
                needed -= take;
            }
            Moments tail=solve_jump_pre(n_rem-freeze_count);
            double e=frozen_sum+tail.ev;
            double e2=frozen_sum*frozen_sum + 2.0*frozen_sum*tail.ev + tail.ev2;
            if(e>best_ev){ best_ev=e; best_ev2=e2; best_fc=freeze_count; }
        }
        best_jump_freeze[{cnt}] = best_fc;
        ev += p*best_ev; ev2 += p*best_ev2;
    }
    return memo_jump[n_rem] = {ev,ev2};
}

static Moments solve_runup_pre(int n_rem, int s){
    if(s>8) return {0.0,0.0};
    if(memo_runup.count({n_rem,s})) return memo_runup[{n_rem,s}];
    int k=5-n_rem;
    Moments stopM=solve_jump_pre(k);
    double ev=0, ev2=0;
    for(auto& pr: outcomes_cache[n_rem]){
        const Counts& cnt=pr.first; double p=pr.second;
        double best_ev=stopM.ev, best_ev2=stopM.ev2; int best_fc=0; // 0 means stop
        for(int freeze_count=1; freeze_count<=n_rem; ++freeze_count){
            // freeze_count smallest dice
            int frozen_sum=0;
            int needed=freeze_count;
            for(int face=1; face<=6 && needed>0; ++face){
                int take=min(cnt.c[face], needed);
                frozen_sum += face*take;
                needed -= take;
            }
            if(s+frozen_sum>8) continue;
            Moments tail=solve_runup_pre(n_rem-freeze_count, s+frozen_sum);
            if(tail.ev>best_ev){ best_ev=tail.ev; best_ev2=tail.ev2; best_fc=freeze_count; }
        }
        best_runup_freeze[{s,cnt}] = best_fc;
        ev += p*best_ev; ev2 += p*best_ev2;
    }
    return memo_runup[{n_rem,s}] = {ev,ev2};
}

int main(int argc,char** argv){
    build_outcomes();
    Moments attemptM=solve_runup_pre(5,0);

    string path=(argc>=2? argv[1] : "longjump_policy_simple.db");
    sqlite3* db=nullptr;
    if(sqlite3_open(path.c_str(),&db)!=SQLITE_OK){ fprintf(stderr,"sqlite open failed\n"); return 1; }

    auto sql_exec=[&](const char* sql){
        char* err=nullptr;
        if(sqlite3_exec(db,sql,nullptr,nullptr,&err)!=SQLITE_OK){
            fprintf(stderr,"sqlite error: %s\n", err?err:"(unknown)"); sqlite3_free(err); exit(2);
        }
    };
    sql_exec("PRAGMA journal_mode=OFF;");
    sql_exec("PRAGMA synchronous=OFF;");
    sql_exec("DROP TABLE IF EXISTS lj_post_simple;");
    sql_exec("DROP TABLE IF EXISTS lj_meta;");
    sql_exec("CREATE TABLE lj_post_simple(phase INTEGER,sum_frozen INTEGER,"
             "n1 INTEGER,n2 INTEGER,n3 INTEGER,n4 INTEGER,n5 INTEGER,n6 INTEGER,"
             "freeze_count INTEGER,"
             "PRIMARY KEY(phase,sum_frozen,n1,n2,n3,n4,n5,n6));");
    sql_exec("CREATE TABLE lj_meta(key TEXT PRIMARY KEY,value REAL);");

    sqlite3_stmt* ins=nullptr;
    string q="INSERT OR REPLACE INTO lj_post_simple VALUES(?,?,?,?,?,?,?,?,?,?,?);";
    if(sqlite3_prepare_v2(db,q.c_str(),-1,&ins,nullptr)!=SQLITE_OK){ fprintf(stderr,"prepare failed\n"); return 2; }
    sql_exec("BEGIN;");
    for(auto& kv: best_runup_freeze){
        sqlite3_reset(ins);
        sqlite3_bind_int(ins,1,RUNUP_POST);
        sqlite3_bind_int(ins,2,kv.first.sum_frozen);
        for(int i=1;i<=6;i++) sqlite3_bind_int(ins,2+i,kv.first.cnt.c[i]);
        sqlite3_bind_int(ins,9,kv.second);
        if(sqlite3_step(ins)!=SQLITE_DONE){ fprintf(stderr,"insert failed\n"); return 3; }
    }
    for(auto& kv: best_jump_freeze){
        sqlite3_reset(ins);
        sqlite3_bind_int(ins,1,JUMP_POST);
        sqlite3_bind_null(ins,2);
        for(int i=1;i<=6;i++) sqlite3_bind_int(ins,2+i,kv.first.cnt.c[i]);
        sqlite3_bind_int(ins,9,kv.second);
        if(sqlite3_step(ins)!=SQLITE_DONE){ fprintf(stderr,"insert failed\n"); return 4; }
    }
    sql_exec("COMMIT;");
    sqlite3_finalize(ins);

    sqlite3_stmt* insm=nullptr;
    if(sqlite3_prepare_v2(db,"INSERT OR REPLACE INTO lj_meta(key,value) VALUES(?,?);",-1,&insm,nullptr)!=SQLITE_OK){
        fprintf(stderr,"prepare meta failed\n"); return 5;
    }
    auto put=[&](const char* k,double v){
        sqlite3_reset(insm);
        sqlite3_bind_text(insm,1,k,-1,SQLITE_TRANSIENT);
        sqlite3_bind_double(insm,2,v);
        if(sqlite3_step(insm)!=SQLITE_DONE){ fprintf(stderr,"insert meta failed\n"); exit(6); }
    };
    put("attempt_ev",attemptM.ev);
    double var=max(0.0,attemptM.ev2 - attemptM.ev*attemptM.ev);
    put("attempt_sd",sqrt(var));
    sqlite3_finalize(insm);
    sqlite3_close(db);

    fprintf(stderr,"Wrote policy to %s (attempt EV=%.6f, SD=%.6f)\n", path.c_str(), attemptM.ev, sqrt(var));
    return 0;
}
