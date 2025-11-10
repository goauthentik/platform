#define STRINGCLASS_CPP
#if defined(WINAPI_FAMILY)
#undef WINAPI_FAMILY
#define WINAPI_FAMILY WINAPI_FAMILY_DESKTOP_APP
#endif
#include "include/ustring.h"
#include "spdlog/spdlog.h"

using std::exception;

ustring::ustring() : wstring()
{
	SPDLOG_DEBUG("ustring()");
	this->allocator = ustring::default_alloc;
	this->releaser = ustring::default_free;
}

ustring::ustring(const wstring & right) : wstring(right)
{
	SPDLOG_DEBUG("ustring(, wstring(, ))");
	this->allocator = ustring::default_alloc;
	this->releaser = ustring::default_free;
}

ustring::ustring(const ustring & right) : wstring(right)
{
	SPDLOG_DEBUG("ustring(, ustring(, ))");
	this->allocator = ustring::default_alloc;
	this->releaser = ustring::default_free;
}

ustring::ustring(const wchar_t * s) : wstring(s)
{
	SPDLOG_DEBUG("ustring(, )");
	this->allocator = ustring::default_alloc;
	this->releaser = ustring::default_free;
}

ustring::~ustring()
{
	SPDLOG_DEBUG("~ustring(, )");
}

void * ustring::default_alloc(size_t s)
{
	return static_cast<void *>(new unsigned char[s]);
}

void ustring::default_free(void * b)
{
	delete[] reinterpret_cast<unsigned char *>(b);
}

UNICODE_STRING * ustring::to_unicode_string()
{
	if(IsBadCodePtr(reinterpret_cast<FARPROC>(this->allocator)))
	{
		// アロケーターが設定されていなければ既定のアロケーターを使用
		this->allocator = ustring::default_alloc;
	}
	if(IsBadCodePtr(reinterpret_cast<FARPROC>(this->releaser)))
	{
		// リリーサーが設定されていなければ既定のリリーサーを使用
		this->releaser = ustring::default_free;
	}

	auto str = static_cast<UNICODE_STRING*>((*this->allocator)(sizeof(UNICODE_STRING)));

	if(!str)
	{
		// UNICODE_STRING構造体のバッファーを取れなかったら例外
		throw exception("[ustring] Can't allocate UNICODE_STRING structure.");
	}

	if(this->empty())
	{
		// 文字列無しだった場合、長さ0のUNICODE_STRINGを返す
		str->Buffer = nullptr;
		str->Length = 0;
		str->MaximumLength = 0;

		return str;
	}

	// UNICODE_STRINGを生成
	{
		// UNICODE_STRING構造体のLengthとMaximumLengthはバイト単位

		// バイト単位で長さを取得し、32766文字以下になるようにする
		auto wsize = sizeof(wchar_t) * this->size();
		if(wsize > UNICODE_STRING_MAX_BYTES)
		{
			wsize = UNICODE_STRING_MAX_BYTES;
		}

		// 32766文字以下に切り詰め
		str->Length = static_cast<USHORT>(wsize);
		str->MaximumLength = static_cast<USHORT>(wsize);

		str->Buffer = static_cast<wchar_t*>((*this->allocator)(wsize));

		if(!str->Buffer)
		{
			// 実際の文字列を保持するバッファーを取れなかった場合は例外
			(*this->releaser)(str);
			throw exception("[ustring] Can't allocate string buffer.");
		}
	}

	// UNICODE_STRINGの文字列にコピー
	memcpy(str->Buffer, this->c_str(), str->Length);

	return str;
}

LSA_STRING * ustring::to_lsa_string()
{
	if(IsBadCodePtr(reinterpret_cast<FARPROC>(this->allocator)))
	{
		// アロケーターが設定されていなければ既定のアロケーターを使用
		this->allocator = ustring::default_alloc;
	}
	if(IsBadCodePtr(reinterpret_cast<FARPROC>(this->releaser)))
	{
		// リリーサーが設定されていなければ例外
		this->releaser = ustring::default_free;
	}

	auto str = static_cast<LSA_STRING*>((*this->allocator)(sizeof(LSA_STRING)));

	if(!str)
	{
		// UNICODE_STRING構造体のバッファーを取れなかったら例外
		throw exception("[ustring] Can't allocate LSA_STRING structure.");
	}

	if(this->empty())
	{
		str->Buffer = nullptr;
		str->Length = 0;
		str->MaximumLength = 0;

		return str;
	}

	auto wsize = static_cast<int>(this->size());

	// UTF-7に変換した際のサイズをチェック
	auto newsize = WideCharToMultiByte(CP_UTF7, 0, this->c_str(), wsize, nullptr, 0, nullptr, nullptr);
	if(newsize <= 0)
	{
		// 変換結果が空文字列の場合は終了
		str->Buffer = nullptr;
		str->Length = 0;
		str->MaximumLength = 0;

		return str;
	}

	if(newsize > USHRT_MAX)
	{
		// LSA_STRINGは65535文字までなので、超えていた場合は切り捨てる
		newsize = USHRT_MAX;
	}

	// LSA_STRINGを生成
	str->Length = newsize;
	str->MaximumLength = newsize;
	str->Buffer = static_cast<char*>((*this->allocator)(newsize));

	if(!str->Buffer)
	{
		// 文字列を格納するバッファーを取れなかった場合はLSA_STRINGを破棄
		(*this->releaser)(str);
		throw exception("[ustring] Can't allocate string buffer.");
	}

	// 変換実行
	WideCharToMultiByte(CP_UTF7, 0, this->c_str(), wsize, str->Buffer, newsize, nullptr, nullptr);

	return str;
}

ustrings::ustrings()
{
	SPDLOG_DEBUG("ustrings()");
}

ustrings::~ustrings()
{
	//this->clear();
	SPDLOG_DEBUG("~ustrings(, %Iu)");
}

void ustrings::push_back(const wchar_t * s)
{
	auto ss = new ustring(s);
	vector::push_back(ss->to_share_ptr());
}

/*void ustrings::clear()
{
	SPDLOG_DEBUG("ustrings()::clear() size=%Iu", this, this->size());

	while(!this->empty())
	{
		auto i = this->back();
		delete i;
		this->pop_back();
	}
}*/
