#include <napi.h>

typedef struct TSLanguage TSLanguage;

extern "C" TSLanguage *tree_sitter_validatetest();

// "tree-sitter", "currentABIVersion" heuristic
static Napi::Number CurrentABIVersion(const Napi::CallbackInfo& info) {
  return Napi::Number::New(info.Env(), NAPI_VERSION);
}

Napi::Object Init(Napi::Env env, Napi::Object exports) {
  exports["name"] = Napi::String::New(env, "validatetest");
  auto language = Napi::External<TSLanguage>::New(env, tree_sitter_validatetest());
  exports["language"] = language;
  exports["currentABIVersion"] = Napi::Function::New(env, CurrentABIVersion);
  return exports;
}

NODE_API_MODULE(tree_sitter_validatetest_binding, Init)
