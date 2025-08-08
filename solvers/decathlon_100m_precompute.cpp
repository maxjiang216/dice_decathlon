// g++ -O3 -std=c++20 solvers/decathlon_100m_precompute.cpp -lsqlite3 -o solvers/100m_precompute
#include <bits/stdc++.h>
#include <sqlite3.h>
using namespace std;

static const int SIDES[6] = {1,2,3,4,5,6};
static vector<array<int,4>> FOUR_OUTS;

inline int score_set(const array<int,4>& d){
    int s=0; for(int v: d) s += (v==6 ? -6 : v); return s;
}

struct State {
    int stage;            // 1 or 2
    int rerolls;          // 0..5
    array<int,4> dice;    // sorted
    int set1_score;       // use 7777 as "NULL" for stage 1
    bool operator==(State const& o) const {
        return stage==o.stage && rerolls==o.rerolls && dice==o.dice && set1_score==o.set1_score;
    }
};

struct StateHash {
    size_t operator()(State const& s) const noexcept {
        size_t h = s.stage*1315423911u + s.rerolls*2654435761u + (unsigned)s.set1_score*97531u;
        for(int v: s.dice) h = h*131u + v;
        return h;
    }
};

struct Moments { double ev=0, ev2=0; };
struct SolveRes {
    Moments best;
    Moments freeze_m;
    optional<Moments> reroll_m; // empty if rerolls=0
    string best_action;         // "freeze" or "reroll"
};

unordered_map<State,SolveRes,StateHash> memo;

static inline Moments combine_avg(const vector<Moments>& ms){
    double ev=0, ev2=0; double w=1.0/ms.size();
    for(auto& m: ms){ ev+=w*m.ev; ev2+=w*m.ev2; }
    return {ev, ev2};
}

SolveRes solve_state(const State& s){
    auto it = memo.find(s);
    if(it!=memo.end()) return it->second;

    // --- Freeze moments
    Moments freeze_m;
    if(s.stage==2){
        int total = s.set1_score + score_set(s.dice);
        freeze_m = {double(total), double(total)*double(total)};
    } else {
        int s1 = score_set(s.dice);
        vector<Moments> kids; kids.reserve(FOUR_OUTS.size());
        for(auto r: FOUR_OUTS){
            sort(r.begin(), r.end());
            SolveRes child = solve_state({2, s.rerolls, r, s1});
            kids.push_back(child.best);
        }
        freeze_m = combine_avg(kids);
    }

    // --- Reroll moments (if any rerolls left)
    optional<Moments> reroll_m;
    if(s.rerolls>0){
        vector<Moments> kids; kids.reserve(FOUR_OUTS.size());
        for(auto r: FOUR_OUTS){
            sort(r.begin(), r.end());
            SolveRes child = solve_state({s.stage, s.rerolls-1, r, s.set1_score});
            kids.push_back(child.best);
        }
        reroll_m = combine_avg(kids);
    }

    // choose best by EV (tie → lower SD → prefer freeze)
    auto sd = [](Moments m){ double var=max(0.0, m.ev2 - m.ev*m.ev); return sqrt(var); };
    string best_action="freeze"; Moments best = freeze_m;
    if(reroll_m){
        double sd_f = sd(freeze_m), sd_r = sd(*reroll_m);
        if ( (reroll_m->ev > freeze_m.ev) ||
             (fabs(reroll_m->ev - freeze_m.ev) < 1e-12 && sd_r < sd_f) )
        { best = *reroll_m; best_action="reroll"; }
    }

    SolveRes res{best, freeze_m, reroll_m, best_action};
    memo.emplace(s, res);
    return res;
}

static void ensure_four_outs(){
    if(!FOUR_OUTS.empty()) return;
    FOUR_OUTS.reserve(1296);
    for(int a: SIDES) for(int b: SIDES) for(int c: SIDES) for(int d: SIDES){
        array<int,4> t{a,b,c,d}; sort(t.begin(), t.end()); FOUR_OUTS.push_back(t);
    }
    // dedup to 126 patterns
    sort(FOUR_OUTS.begin(), FOUR_OUTS.end());
    FOUR_OUTS.erase(unique(FOUR_OUTS.begin(), FOUR_OUTS.end()), FOUR_OUTS.end());
}

static void sql_exec(sqlite3* db, const char* sql){
    char* err=nullptr;
    if(sqlite3_exec(db, sql, nullptr, nullptr, &err)!=SQLITE_OK){
        string msg = err?err:"(unknown sqlite error)";
        sqlite3_free(err);
        throw runtime_error("sqlite error: " + msg);
    }
}

int main(int argc, char** argv){
    ensure_four_outs();
    // pre-warm recursion by solving everything
    // enumerate all states
    vector<State> all;
    all.reserve(50000);

    // stage 1
    for(int r=0;r<=5;r++)
        for(auto d: FOUR_OUTS)
            all.push_back({1,r,d,7777}); // 7777 sentinel = NULL

    // stage 2
    for(int r=0;r<=5;r++)
        for(auto d: FOUR_OUTS)
            for(int s1=-24; s1<=20; ++s1)
                all.push_back({2,r,d,s1});

    // Solve all (memoized recursion – super fast)
    for(auto& s: all) solve_state(s);

    // open DB
    string path = (argc>=2? argv[1] : "100m_policy.db");
    sqlite3* db=nullptr;
    if(sqlite3_open(path.c_str(), &db)!=SQLITE_OK) throw runtime_error("sqlite open failed");

    sql_exec(db, "PRAGMA journal_mode=OFF;");
    sql_exec(db, "PRAGMA synchronous=OFF;");
    sql_exec(db, "DROP TABLE IF EXISTS states100m;");
    sql_exec(db,
        "CREATE TABLE states100m ("
        " stage INTEGER NOT NULL,"
        " rerolls INTEGER NOT NULL,"
        " d1 INTEGER NOT NULL, d2 INTEGER NOT NULL, d3 INTEGER NOT NULL, d4 INTEGER NOT NULL,"
        " set1_score INTEGER,"    // NULL for stage 1
        " ev_freeze REAL NOT NULL, sd_freeze REAL NOT NULL,"
        " ev_reroll REAL, sd_reroll REAL,"
        " best TEXT NOT NULL,"
        " PRIMARY KEY (stage,rerolls,d1,d2,d3,d4,set1_score)"
        ");"
    );

    sqlite3_stmt* ins=nullptr;
    string q = "INSERT INTO states100m "
               "(stage,rerolls,d1,d2,d3,d4,set1_score,ev_freeze,sd_freeze,ev_reroll,sd_reroll,best) "
               "VALUES (?,?,?,?,?,?,?,?,?,?,?,?);";
    if(sqlite3_prepare_v2(db, q.c_str(), -1, &ins, nullptr)!=SQLITE_OK) throw runtime_error("prepare failed");

    auto sd = [](Moments m){ double var=max(0.0, m.ev2 - m.ev*m.ev); return sqrt(var); };
    sql_exec(db, "BEGIN;");
    for(auto& kv : memo){
        const State& s = kv.first;
        const SolveRes& r = kv.second;
        sqlite3_reset(ins);
        sqlite3_bind_int  (ins, 1, s.stage);
        sqlite3_bind_int  (ins, 2, s.rerolls);
        sqlite3_bind_int  (ins, 3, s.dice[0]);
        sqlite3_bind_int  (ins, 4, s.dice[1]);
        sqlite3_bind_int  (ins, 5, s.dice[2]);
        sqlite3_bind_int  (ins, 6, s.dice[3]);
        if(s.stage==1) sqlite3_bind_null(ins, 7);
        else           sqlite3_bind_int (ins, 7, s.set1_score);

        sqlite3_bind_double(ins, 8, r.freeze_m.ev);
        sqlite3_bind_double(ins, 9, sd(r.freeze_m));

        if(r.reroll_m){
            sqlite3_bind_double(ins,10, r.reroll_m->ev);
            sqlite3_bind_double(ins,11, sd(*r.reroll_m));
        }else{
            sqlite3_bind_null  (ins,10);
            sqlite3_bind_null  (ins,11);
        }
        sqlite3_bind_text  (ins,12, r.best_action.c_str(), -1, SQLITE_TRANSIENT);

        if(sqlite3_step(ins)!=SQLITE_DONE) throw runtime_error("insert failed");
    }
    sql_exec(db, "COMMIT;");
    sqlite3_finalize(ins);
    sql_exec(db, "CREATE INDEX IF NOT EXISTS idx_states100m ON states100m(stage,rerolls,d1,d2,d3,d4,set1_score);");
    sqlite3_close(db);

    cerr << "Wrote " << memo.size() << " states to " << path << "\n";
    return 0;
}
