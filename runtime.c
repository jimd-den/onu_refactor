#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/// The Ọ̀nụ Runtime Infrastructure (The Detail)

typedef struct {
    long long len;
    char* data;
} String;

typedef struct MapEntry {
    void* key;
    void* value;
    unsigned long long hash;
    struct MapEntry* next;
} MapEntry;

typedef struct {
    long long size;
    long long capacity;
    MapEntry** buckets;
} Map;

typedef struct TreeNode {
    void* value;
    struct TreeNode* left;
    struct TreeNode* right;
    long long height;
} TreeNode;

typedef struct {
    TreeNode* root;
    long long size;
} Tree;

void* onu_malloc(size_t size) {
    return malloc(size);
}

void onu_free(void* ptr) {
    if (ptr != NULL) free(ptr);
}

String onu_create_string(const char* s) {
    size_t len = strlen(s);
    char* data = onu_malloc(len + 1);
    memcpy(data, s, len + 1);
    String res = {(long long)len, data};
    return res;
}

String as_text(long long n) __asm__("as-text");
String joined_with(String a, String b) __asm__("joined-with");
long long onu_len(String s) __asm__("len");
long long onu_char_at(String s, long long idx) __asm__("char-at");
String onu_init_of(String s) __asm__("init-of");
String onu_char_from_code(long long code) __asm__("char-from-code");
String onu_strdup(String s) __asm__("duplicated-as");
String onu_set_char(String s, long long idx, long long val) __asm__("set-char");
String onu_inplace_set_char(String s, long long idx, long long val) __asm__("inplace-set-char");

String as_text(long long n) {
    char* buf = onu_malloc(32);
    sprintf(buf, "%lld", n);
    String res = {(long long)strlen(buf), buf};
    return res;
}

String joined_with(String a, String b) {
    char* res_data = onu_malloc(a.len + b.len + 1);
    memcpy(res_data, a.data, a.len);
    memcpy(res_data + a.len, b.data, b.len);
    res_data[a.len + b.len] = '\0';
    String res = {a.len + b.len, res_data};
    return res;
}

long long onu_len(String s) {
    return s.len;
}

long long onu_char_at(String s, long long idx) {
    if (idx < 0 || idx >= s.len) return 0;
    return (long long)s.data[idx];
}

String onu_init_of(String s) {
    if (s.len <= 1) {
        char* empty = onu_malloc(1);
        empty[0] = '\0';
        String res = {0, empty};
        return res;
    }
    char* res_data = onu_malloc(s.len);
    memcpy(res_data, s.data, s.len - 1);
    res_data[s.len - 1] = '\0';
    String res = {s.len - 1, res_data};
    return res;
}

String onu_char_from_code(long long code) {
    char* res_data = onu_malloc(2);
    res_data[0] = (char)code;
    res_data[1] = '\0';
    String res = {1, res_data};
    return res;
}

String onu_strdup(String s) {
    char* res_data = onu_malloc(s.len + 1);
    memcpy(res_data, s.data, s.len + 1);
    String res = {s.len, res_data};
    return res;
}

String onu_set_char(String s, long long idx, long long val) {
    char* res_data = onu_malloc(s.len + 1);
    memcpy(res_data, s.data, s.len + 1);
    if (idx >= 0 && idx < s.len) {
        res_data[idx] = (char)val;
    }
    String res = {s.len, res_data};
    return res;
}

String onu_inplace_set_char(String s, long long idx, long long val) {
    if (idx >= 0 && idx < s.len) {
        s.data[idx] = (char)val;
    }
    return s;
}

unsigned long long onu_hash(void* key, int is_string) {
    if (is_string) {
        String* s = (String*)key;
        unsigned long long h = 14695981039346656037ULL;
        for (long long i = 0; i < s->len; i++) {
            h ^= (unsigned char)s->data[i];
            h *= 1099511628211ULL;
        }
        return h;
    } else {
        return (unsigned long long)key;
    }
}

Map* onu_map_create(long long capacity) {
    Map* map = onu_malloc(sizeof(Map));
    map->size = 0;
    map->capacity = capacity > 0 ? capacity : 16;
    map->buckets = onu_malloc(sizeof(MapEntry*) * map->capacity);
    memset(map->buckets, 0, sizeof(MapEntry*) * map->capacity);
    return map;
}

void onu_map_resize(Map* map) {
    long long old_capacity = map->capacity;
    MapEntry** old_buckets = map->buckets;
    
    map->capacity *= 2;
    map->buckets = onu_malloc(sizeof(MapEntry*) * map->capacity);
    memset(map->buckets, 0, sizeof(MapEntry*) * map->capacity);
    
    for (long long i = 0; i < old_capacity; i++) {
        MapEntry* entry = old_buckets[i];
        while (entry) {
            MapEntry* next = entry->next;
            long long new_idx = entry->hash % map->capacity;
            entry->next = map->buckets[new_idx];
            map->buckets[new_idx] = entry;
            entry = next;
        }
    }
    onu_free(old_buckets);
}

Map* onu_map_insert(Map* map, void* key, void* value, int is_string) {
    if (map->size >= map->capacity * 0.75) {
        onu_map_resize(map);
    }
    unsigned long long h = onu_hash(key, is_string);
    long long idx = h % map->capacity;
    
    MapEntry* entry = map->buckets[idx];
    while (entry) {
        int match = 0;
        if (is_string) {
            String* s1 = (String*)key;
            String* s2 = (String*)entry->key;
            if (s1->len == s2->len && memcmp(s1->data, s2->data, s1->len) == 0) match = 1;
        } else {
            if (entry->key == key) match = 1;
        }
        
        if (match) {
            entry->value = value;
            return map;
        }
        entry = entry->next;
    }
    
    MapEntry* new_entry = onu_malloc(sizeof(MapEntry));
    new_entry->key = key;
    new_entry->value = value;
    new_entry->hash = h;
    new_entry->next = map->buckets[idx];
    map->buckets[idx] = new_entry;
    map->size++;
    return map;
}

void* onu_map_find(Map* map, void* key, int is_string) {
    unsigned long long h = onu_hash(key, is_string);
    long long idx = h % map->capacity;
    MapEntry* entry = map->buckets[idx];
    while (entry) {
        int match = 0;
        if (is_string) {
            String* s1 = (String*)key;
            String* s2 = (String*)entry->key;
            if (s1->len == s2->len && memcmp(s1->data, s2->data, s1->len) == 0) match = 1;
        } else {
            if (entry->key == key) match = 1;
        }
        if (match) return entry->value;
        entry = entry->next;
    }
    return NULL;
}

long long onu_max(long long a, long long b) { return a > b ? a : b; }
long long onu_get_height(TreeNode* n) { return n ? n->height : 0; }

TreeNode* onu_rotate_right(TreeNode* y) {
    TreeNode* x = y->left;
    TreeNode* T2 = x->right;
    x->right = y;
    y->left = T2;
    y->height = onu_max(onu_get_height(y->left), onu_get_height(y->right)) + 1;
    x->height = onu_max(onu_get_height(x->left), onu_get_height(x->right)) + 1;
    return x;
}

TreeNode* onu_rotate_left(TreeNode* x) {
    TreeNode* y = x->right;
    TreeNode* T2 = y->left;
    y->left = x;
    x->right = T2;
    x->height = onu_max(onu_get_height(x->left), onu_get_height(x->right)) + 1;
    y->height = onu_max(onu_get_height(y->left), onu_get_height(y->right)) + 1;
    return y;
}

TreeNode* onu_tree_insert_node(TreeNode* node, void* value) {
    if (node == NULL) {
        TreeNode* n = onu_malloc(sizeof(TreeNode));
        n->value = value;
        n->left = n->right = NULL;
        n->height = 1;
        return n;
    }
    
    if ((long long)value < (long long)node->value)
        node->left = onu_tree_insert_node(node->left, value);
    else if ((long long)value > (long long)node->value)
        node->right = onu_tree_insert_node(node->right, value);
    else
        return node;

    node->height = 1 + onu_max(onu_get_height(node->left), onu_get_height(node->right));
    long long balance = onu_get_height(node->left) - onu_get_height(node->right);

    if (balance > 1 && (long long)value < (long long)node->left->value)
        return onu_rotate_right(node);
    if (balance < -1 && (long long)value > (long long)node->right->value)
        return onu_rotate_left(node);
    if (balance > 1 && (long long)value > (long long)node->left->value) {
        node->left = onu_rotate_left(node->left);
        return onu_rotate_right(node);
    }
    if (balance < -1 && (long long)value < (long long)node->right->value) {
        node->right = onu_rotate_right(node->right);
        return onu_rotate_left(node);
    }
    return node;
}

Tree* onu_tree_create() {
    Tree* tree = onu_malloc(sizeof(Tree));
    tree->root = NULL;
    tree->size = 0;
    return tree;
}

Tree* onu_tree_insert(Tree* tree, void* value) {
    tree->root = onu_tree_insert_node(tree->root, value);
    tree->size++;
    return tree;
}

static int global_argc = 0;
static char** global_argv = NULL;

void onu_init_args(int argc, char** argv) {
    global_argc = argc;
    global_argv = argv;
}

long long onu_get_arg_count() {
    return (long long)global_argc;
}

String onu_get_arg(long long index) {
    if (index < 0 || index >= global_argc) {
        String empty = {0, ""};
        return empty;
    }
    return onu_create_string(global_argv[index]);
}

String onu_receives_line() __asm__("receives-line");
long long onu_as_integer(String s) __asm__("as-integer");
long long onu_receives_entropy() __asm__("receives-entropy");

String onu_receives_line() {
    char buf[1024];
    if (fgets(buf, sizeof(buf), stdin)) {
        size_t len = strlen(buf);
        if (len > 0 && buf[len-1] == '\n') {
            buf[len-1] = '\0';
            len--;
        }
        return onu_create_string(buf);
    }
    return onu_create_string("");
}

long long onu_as_integer(String s) {
    return atoll(s.data);
}

#include <time.h>
long long onu_receives_entropy() {
    static int initialized = 0;
    if (!initialized) {
        srand(time(NULL));
        initialized = 1;
    }
    return (long long)rand();
}

#include <unistd.h>

void onu_sleep(long long ms) {
    usleep(ms * 1000);
}

void broadcasts(const char* s) {
    if (s != NULL) {
        puts(s);
        fflush(stdout);
    }
}
