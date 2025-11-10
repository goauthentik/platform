#pragma once

#include <string>
#include <vector>
#include <memory>
#include <Windows.h>
#include <NTSecAPI.h>

using std::default_delete;
using std::wstring;
using std::vector;
using std::shared_ptr;

typedef void * (* ustring_alloc)(size_t);
typedef void   (* ustring_free )(void *);

#if defined(STRINGCLASS_CPP)
template class shared_ptr<class ustring>;
template class vector<shared_ptr<class ustring>>;
#else
extern template class shared_ptr<class ustring>;
extern template class vector<shared_ptr<class ustring>>;
#endif

class ustring : public wstring
{
private:
	ustring_alloc allocator;
	ustring_free  releaser;
public:
	ustring();
	ustring(const wstring &);
	ustring(const ustring &);
	ustring(const wchar_t *);
	virtual ~ustring();

	inline void set_allocater(ustring_alloc a, ustring_free f) { this->allocator = a; this->releaser = f; }

	inline operator size_t() { return this->size(); }
	inline operator const wchar_t *() { return this->c_str(); }

	UNICODE_STRING * to_unicode_string();
	inline operator UNICODE_STRING *() { return this->to_unicode_string(); }

	LSA_STRING * to_lsa_string();
	inline operator LSA_STRING *() { return this->to_lsa_string(); }

	inline shared_ptr<ustring> to_share_ptr() { return shared_ptr<ustring>(this); }
	inline operator shared_ptr<ustring>() { return this->to_share_ptr(); }

	static void * default_alloc(size_t);
	static void default_free(void *);
};

class ustrings : public vector<shared_ptr<ustring>>
{
public:
	ustrings();
	virtual ~ustrings();

	void push_back(const wchar_t *);
};
