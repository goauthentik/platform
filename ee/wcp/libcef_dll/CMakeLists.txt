# Copyright (c) 2014 The Chromium Embedded Framework Authors. All rights
# reserved. Use of this source code is governed by a BSD-style license that
# can be found in the LICENSE file.

# Append platform specific sources to a list of sources.
macro(LIBCEF_APPEND_PLATFORM_SOURCES name_of_list)
  if("${CMAKE_SYSTEM_NAME}" STREQUAL "Darwin" AND ${name_of_list}_MAC)
    list(APPEND ${name_of_list} ${${name_of_list}_MAC})
  endif()
  if("${CMAKE_SYSTEM_NAME}" STREQUAL "Linux" AND ${name_of_list}_LINUX)
    list(APPEND ${name_of_list} ${${name_of_list}_LINUX})
  endif()
  if("${CMAKE_SYSTEM_NAME}" STREQUAL "Windows" AND ${name_of_list}_WINDOWS)
    list(APPEND ${name_of_list} ${${name_of_list}_WINDOWS})
  endif()
endmacro()

set(CEF_TARGET libcef_dll_wrapper)

set(LIBCEF_SRCS
  shutdown_checker.cc
  shutdown_checker.h
  template_util.h
  transfer_util.cc
  transfer_util.h
  wrapper_types.h
  )
source_group(libcef_dll FILES ${LIBCEF_SRCS})

set(LIBCEF_BASE_SRCS
  base/cef_atomic_flag.cc
  base/cef_callback_helpers.cc
  base/cef_callback_internal.cc
  base/cef_dump_without_crashing.cc
  base/cef_lock.cc
  base/cef_lock_impl.cc
  base/cef_logging.cc
  base/cef_ref_counted.cc
  base/cef_thread_checker_impl.cc
  base/cef_weak_ptr.cc
  )
source_group(libcef_dll\\\\base FILES ${LIBCEF_BASE_SRCS})

set(LIBCEF_CPPTOC_SRCS
  cpptoc/accessibility_handler_cpptoc.cc
  cpptoc/accessibility_handler_cpptoc.h
  cpptoc/app_cpptoc.cc
  cpptoc/app_cpptoc.h
  cpptoc/audio_handler_cpptoc.cc
  cpptoc/audio_handler_cpptoc.h
  cpptoc/base_ref_counted_cpptoc.cc
  cpptoc/base_ref_counted_cpptoc.h
  cpptoc/base_scoped_cpptoc.cc
  cpptoc/base_scoped_cpptoc.h
  cpptoc/browser_process_handler_cpptoc.cc
  cpptoc/browser_process_handler_cpptoc.h
  cpptoc/client_cpptoc.cc
  cpptoc/client_cpptoc.h
  cpptoc/command_handler_cpptoc.cc
  cpptoc/command_handler_cpptoc.h
  cpptoc/completion_callback_cpptoc.cc
  cpptoc/completion_callback_cpptoc.h
  cpptoc/context_menu_handler_cpptoc.cc
  cpptoc/context_menu_handler_cpptoc.h
  cpptoc/cookie_access_filter_cpptoc.cc
  cpptoc/cookie_access_filter_cpptoc.h
  cpptoc/cookie_visitor_cpptoc.cc
  cpptoc/cookie_visitor_cpptoc.h
  cpptoc/cpptoc_ref_counted.h
  cpptoc/cpptoc_scoped.h
  cpptoc/delete_cookies_callback_cpptoc.cc
  cpptoc/delete_cookies_callback_cpptoc.h
  cpptoc/dev_tools_message_observer_cpptoc.cc
  cpptoc/dev_tools_message_observer_cpptoc.h
  cpptoc/dialog_handler_cpptoc.cc
  cpptoc/dialog_handler_cpptoc.h
  cpptoc/display_handler_cpptoc.cc
  cpptoc/display_handler_cpptoc.h
  cpptoc/domvisitor_cpptoc.cc
  cpptoc/domvisitor_cpptoc.h
  cpptoc/download_handler_cpptoc.cc
  cpptoc/download_handler_cpptoc.h
  cpptoc/download_image_callback_cpptoc.cc
  cpptoc/download_image_callback_cpptoc.h
  cpptoc/drag_handler_cpptoc.cc
  cpptoc/drag_handler_cpptoc.h
  cpptoc/end_tracing_callback_cpptoc.cc
  cpptoc/end_tracing_callback_cpptoc.h
  cpptoc/find_handler_cpptoc.cc
  cpptoc/find_handler_cpptoc.h
  cpptoc/focus_handler_cpptoc.cc
  cpptoc/focus_handler_cpptoc.h
  cpptoc/frame_handler_cpptoc.cc
  cpptoc/frame_handler_cpptoc.h
  cpptoc/jsdialog_handler_cpptoc.cc
  cpptoc/jsdialog_handler_cpptoc.h
  cpptoc/keyboard_handler_cpptoc.cc
  cpptoc/keyboard_handler_cpptoc.h
  cpptoc/life_span_handler_cpptoc.cc
  cpptoc/life_span_handler_cpptoc.h
  cpptoc/load_handler_cpptoc.cc
  cpptoc/load_handler_cpptoc.h
  cpptoc/media_observer_cpptoc.cc
  cpptoc/media_observer_cpptoc.h
  cpptoc/media_route_create_callback_cpptoc.cc
  cpptoc/media_route_create_callback_cpptoc.h
  cpptoc/media_sink_device_info_callback_cpptoc.cc
  cpptoc/media_sink_device_info_callback_cpptoc.h
  cpptoc/menu_model_delegate_cpptoc.cc
  cpptoc/menu_model_delegate_cpptoc.h
  cpptoc/navigation_entry_visitor_cpptoc.cc
  cpptoc/navigation_entry_visitor_cpptoc.h
  cpptoc/pdf_print_callback_cpptoc.cc
  cpptoc/pdf_print_callback_cpptoc.h
  cpptoc/permission_handler_cpptoc.cc
  cpptoc/permission_handler_cpptoc.h
  cpptoc/preference_observer_cpptoc.cc
  cpptoc/preference_observer_cpptoc.h
  cpptoc/print_handler_cpptoc.cc
  cpptoc/print_handler_cpptoc.h
  cpptoc/read_handler_cpptoc.cc
  cpptoc/read_handler_cpptoc.h
  cpptoc/render_handler_cpptoc.cc
  cpptoc/render_handler_cpptoc.h
  cpptoc/render_process_handler_cpptoc.cc
  cpptoc/render_process_handler_cpptoc.h
  cpptoc/request_context_handler_cpptoc.cc
  cpptoc/request_context_handler_cpptoc.h
  cpptoc/request_handler_cpptoc.cc
  cpptoc/request_handler_cpptoc.h
  cpptoc/resolve_callback_cpptoc.cc
  cpptoc/resolve_callback_cpptoc.h
  cpptoc/resource_bundle_handler_cpptoc.cc
  cpptoc/resource_bundle_handler_cpptoc.h
  cpptoc/resource_handler_cpptoc.cc
  cpptoc/resource_handler_cpptoc.h
  cpptoc/resource_request_handler_cpptoc.cc
  cpptoc/resource_request_handler_cpptoc.h
  cpptoc/response_filter_cpptoc.cc
  cpptoc/response_filter_cpptoc.h
  cpptoc/run_file_dialog_callback_cpptoc.cc
  cpptoc/run_file_dialog_callback_cpptoc.h
  cpptoc/scheme_handler_factory_cpptoc.cc
  cpptoc/scheme_handler_factory_cpptoc.h
  cpptoc/server_handler_cpptoc.cc
  cpptoc/server_handler_cpptoc.h
  cpptoc/set_cookie_callback_cpptoc.cc
  cpptoc/set_cookie_callback_cpptoc.h
  cpptoc/setting_observer_cpptoc.cc
  cpptoc/setting_observer_cpptoc.h
  cpptoc/string_visitor_cpptoc.cc
  cpptoc/string_visitor_cpptoc.h
  cpptoc/task_cpptoc.cc
  cpptoc/task_cpptoc.h
  cpptoc/urlrequest_client_cpptoc.cc
  cpptoc/urlrequest_client_cpptoc.h
  cpptoc/v8_accessor_cpptoc.cc
  cpptoc/v8_accessor_cpptoc.h
  cpptoc/v8_array_buffer_release_callback_cpptoc.cc
  cpptoc/v8_array_buffer_release_callback_cpptoc.h
  cpptoc/v8_handler_cpptoc.cc
  cpptoc/v8_handler_cpptoc.h
  cpptoc/v8_interceptor_cpptoc.cc
  cpptoc/v8_interceptor_cpptoc.h
  cpptoc/write_handler_cpptoc.cc
  cpptoc/write_handler_cpptoc.h
  )
source_group(libcef_dll\\\\cpptoc FILES ${LIBCEF_CPPTOC_SRCS})

set(LIBCEF_CPPTOC_TEST_SRCS
  cpptoc/test/api_version_test_ref_ptr_client_child_cpptoc.cc
  cpptoc/test/api_version_test_ref_ptr_client_child_cpptoc.h
  cpptoc/test/api_version_test_ref_ptr_client_child_v2_cpptoc.cc
  cpptoc/test/api_version_test_ref_ptr_client_child_v2_cpptoc.h
  cpptoc/test/api_version_test_ref_ptr_client_cpptoc.cc
  cpptoc/test/api_version_test_ref_ptr_client_cpptoc.h
  cpptoc/test/api_version_test_scoped_client_child_cpptoc.cc
  cpptoc/test/api_version_test_scoped_client_child_cpptoc.h
  cpptoc/test/api_version_test_scoped_client_child_v2_cpptoc.cc
  cpptoc/test/api_version_test_scoped_client_child_v2_cpptoc.h
  cpptoc/test/api_version_test_scoped_client_cpptoc.cc
  cpptoc/test/api_version_test_scoped_client_cpptoc.h
  cpptoc/test/test_server_handler_cpptoc.cc
  cpptoc/test/test_server_handler_cpptoc.h
  cpptoc/test/translator_test_ref_ptr_client_child_cpptoc.cc
  cpptoc/test/translator_test_ref_ptr_client_child_cpptoc.h
  cpptoc/test/translator_test_ref_ptr_client_cpptoc.cc
  cpptoc/test/translator_test_ref_ptr_client_cpptoc.h
  cpptoc/test/translator_test_scoped_client_child_cpptoc.cc
  cpptoc/test/translator_test_scoped_client_child_cpptoc.h
  cpptoc/test/translator_test_scoped_client_cpptoc.cc
  cpptoc/test/translator_test_scoped_client_cpptoc.h
  )
source_group(libcef_dll\\\\cpptoc\\\\test FILES ${LIBCEF_CPPTOC_TEST_SRCS})

set(LIBCEF_CPPTOC_VIEWS_SRCS
  cpptoc/views/browser_view_delegate_cpptoc.cc
  cpptoc/views/browser_view_delegate_cpptoc.h
  cpptoc/views/button_delegate_cpptoc.cc
  cpptoc/views/button_delegate_cpptoc.h
  cpptoc/views/menu_button_delegate_cpptoc.cc
  cpptoc/views/menu_button_delegate_cpptoc.h
  cpptoc/views/panel_delegate_cpptoc.cc
  cpptoc/views/panel_delegate_cpptoc.h
  cpptoc/views/textfield_delegate_cpptoc.cc
  cpptoc/views/textfield_delegate_cpptoc.h
  cpptoc/views/view_delegate_cpptoc.cc
  cpptoc/views/view_delegate_cpptoc.h
  cpptoc/views/window_delegate_cpptoc.cc
  cpptoc/views/window_delegate_cpptoc.h
  )
source_group(libcef_dll\\\\cpptoc\\\\views FILES ${LIBCEF_CPPTOC_VIEWS_SRCS})

set(LIBCEF_CTOCPP_SRCS
  ctocpp/auth_callback_ctocpp.cc
  ctocpp/auth_callback_ctocpp.h
  ctocpp/before_download_callback_ctocpp.cc
  ctocpp/before_download_callback_ctocpp.h
  ctocpp/binary_value_ctocpp.cc
  ctocpp/binary_value_ctocpp.h
  ctocpp/browser_ctocpp.cc
  ctocpp/browser_ctocpp.h
  ctocpp/browser_host_ctocpp.cc
  ctocpp/browser_host_ctocpp.h
  ctocpp/callback_ctocpp.cc
  ctocpp/callback_ctocpp.h
  ctocpp/command_line_ctocpp.cc
  ctocpp/command_line_ctocpp.h
  ctocpp/context_menu_params_ctocpp.cc
  ctocpp/context_menu_params_ctocpp.h
  ctocpp/cookie_manager_ctocpp.cc
  ctocpp/cookie_manager_ctocpp.h
  ctocpp/ctocpp_ref_counted.h
  ctocpp/ctocpp_scoped.h
  ctocpp/dictionary_value_ctocpp.cc
  ctocpp/dictionary_value_ctocpp.h
  ctocpp/domdocument_ctocpp.cc
  ctocpp/domdocument_ctocpp.h
  ctocpp/domnode_ctocpp.cc
  ctocpp/domnode_ctocpp.h
  ctocpp/download_item_callback_ctocpp.cc
  ctocpp/download_item_callback_ctocpp.h
  ctocpp/download_item_ctocpp.cc
  ctocpp/download_item_ctocpp.h
  ctocpp/drag_data_ctocpp.cc
  ctocpp/drag_data_ctocpp.h
  ctocpp/file_dialog_callback_ctocpp.cc
  ctocpp/file_dialog_callback_ctocpp.h
  ctocpp/frame_ctocpp.cc
  ctocpp/frame_ctocpp.h
  ctocpp/image_ctocpp.cc
  ctocpp/image_ctocpp.h
  ctocpp/jsdialog_callback_ctocpp.cc
  ctocpp/jsdialog_callback_ctocpp.h
  ctocpp/list_value_ctocpp.cc
  ctocpp/list_value_ctocpp.h
  ctocpp/media_access_callback_ctocpp.cc
  ctocpp/media_access_callback_ctocpp.h
  ctocpp/media_route_ctocpp.cc
  ctocpp/media_route_ctocpp.h
  ctocpp/media_router_ctocpp.cc
  ctocpp/media_router_ctocpp.h
  ctocpp/media_sink_ctocpp.cc
  ctocpp/media_sink_ctocpp.h
  ctocpp/media_source_ctocpp.cc
  ctocpp/media_source_ctocpp.h
  ctocpp/menu_model_ctocpp.cc
  ctocpp/menu_model_ctocpp.h
  ctocpp/navigation_entry_ctocpp.cc
  ctocpp/navigation_entry_ctocpp.h
  ctocpp/permission_prompt_callback_ctocpp.cc
  ctocpp/permission_prompt_callback_ctocpp.h
  ctocpp/post_data_ctocpp.cc
  ctocpp/post_data_ctocpp.h
  ctocpp/post_data_element_ctocpp.cc
  ctocpp/post_data_element_ctocpp.h
  ctocpp/preference_manager_ctocpp.cc
  ctocpp/preference_manager_ctocpp.h
  ctocpp/preference_registrar_ctocpp.cc
  ctocpp/preference_registrar_ctocpp.h
  ctocpp/print_dialog_callback_ctocpp.cc
  ctocpp/print_dialog_callback_ctocpp.h
  ctocpp/print_job_callback_ctocpp.cc
  ctocpp/print_job_callback_ctocpp.h
  ctocpp/print_settings_ctocpp.cc
  ctocpp/print_settings_ctocpp.h
  ctocpp/process_message_ctocpp.cc
  ctocpp/process_message_ctocpp.h
  ctocpp/registration_ctocpp.cc
  ctocpp/registration_ctocpp.h
  ctocpp/request_context_ctocpp.cc
  ctocpp/request_context_ctocpp.h
  ctocpp/request_ctocpp.cc
  ctocpp/request_ctocpp.h
  ctocpp/resource_bundle_ctocpp.cc
  ctocpp/resource_bundle_ctocpp.h
  ctocpp/resource_read_callback_ctocpp.cc
  ctocpp/resource_read_callback_ctocpp.h
  ctocpp/resource_skip_callback_ctocpp.cc
  ctocpp/resource_skip_callback_ctocpp.h
  ctocpp/response_ctocpp.cc
  ctocpp/response_ctocpp.h
  ctocpp/run_context_menu_callback_ctocpp.cc
  ctocpp/run_context_menu_callback_ctocpp.h
  ctocpp/run_quick_menu_callback_ctocpp.cc
  ctocpp/run_quick_menu_callback_ctocpp.h
  ctocpp/scheme_registrar_ctocpp.cc
  ctocpp/scheme_registrar_ctocpp.h
  ctocpp/select_client_certificate_callback_ctocpp.cc
  ctocpp/select_client_certificate_callback_ctocpp.h
  ctocpp/server_ctocpp.cc
  ctocpp/server_ctocpp.h
  ctocpp/shared_memory_region_ctocpp.cc
  ctocpp/shared_memory_region_ctocpp.h
  ctocpp/shared_process_message_builder_ctocpp.cc
  ctocpp/shared_process_message_builder_ctocpp.h
  ctocpp/sslinfo_ctocpp.cc
  ctocpp/sslinfo_ctocpp.h
  ctocpp/sslstatus_ctocpp.cc
  ctocpp/sslstatus_ctocpp.h
  ctocpp/stream_reader_ctocpp.cc
  ctocpp/stream_reader_ctocpp.h
  ctocpp/stream_writer_ctocpp.cc
  ctocpp/stream_writer_ctocpp.h
  ctocpp/task_manager_ctocpp.cc
  ctocpp/task_manager_ctocpp.h
  ctocpp/task_runner_ctocpp.cc
  ctocpp/task_runner_ctocpp.h
  ctocpp/thread_ctocpp.cc
  ctocpp/thread_ctocpp.h
  ctocpp/unresponsive_process_callback_ctocpp.cc
  ctocpp/unresponsive_process_callback_ctocpp.h
  ctocpp/urlrequest_ctocpp.cc
  ctocpp/urlrequest_ctocpp.h
  ctocpp/v8_context_ctocpp.cc
  ctocpp/v8_context_ctocpp.h
  ctocpp/v8_exception_ctocpp.cc
  ctocpp/v8_exception_ctocpp.h
  ctocpp/v8_stack_frame_ctocpp.cc
  ctocpp/v8_stack_frame_ctocpp.h
  ctocpp/v8_stack_trace_ctocpp.cc
  ctocpp/v8_stack_trace_ctocpp.h
  ctocpp/v8_value_ctocpp.cc
  ctocpp/v8_value_ctocpp.h
  ctocpp/value_ctocpp.cc
  ctocpp/value_ctocpp.h
  ctocpp/waitable_event_ctocpp.cc
  ctocpp/waitable_event_ctocpp.h
  ctocpp/x509_cert_principal_ctocpp.cc
  ctocpp/x509_cert_principal_ctocpp.h
  ctocpp/x509_certificate_ctocpp.cc
  ctocpp/x509_certificate_ctocpp.h
  ctocpp/xml_reader_ctocpp.cc
  ctocpp/xml_reader_ctocpp.h
  ctocpp/zip_reader_ctocpp.cc
  ctocpp/zip_reader_ctocpp.h
  )
source_group(libcef_dll\\\\ctocpp FILES ${LIBCEF_CTOCPP_SRCS})

set(LIBCEF_CTOCPP_TEST_SRCS
  ctocpp/test/api_version_test_ctocpp.cc
  ctocpp/test/api_version_test_ctocpp.h
  ctocpp/test/api_version_test_ref_ptr_library_child_child_ctocpp.cc
  ctocpp/test/api_version_test_ref_ptr_library_child_child_ctocpp.h
  ctocpp/test/api_version_test_ref_ptr_library_child_child_v1_ctocpp.cc
  ctocpp/test/api_version_test_ref_ptr_library_child_child_v1_ctocpp.h
  ctocpp/test/api_version_test_ref_ptr_library_child_child_v2_ctocpp.cc
  ctocpp/test/api_version_test_ref_ptr_library_child_child_v2_ctocpp.h
  ctocpp/test/api_version_test_ref_ptr_library_child_ctocpp.cc
  ctocpp/test/api_version_test_ref_ptr_library_child_ctocpp.h
  ctocpp/test/api_version_test_ref_ptr_library_ctocpp.cc
  ctocpp/test/api_version_test_ref_ptr_library_ctocpp.h
  ctocpp/test/api_version_test_scoped_library_child_child_ctocpp.cc
  ctocpp/test/api_version_test_scoped_library_child_child_ctocpp.h
  ctocpp/test/api_version_test_scoped_library_child_child_v1_ctocpp.cc
  ctocpp/test/api_version_test_scoped_library_child_child_v1_ctocpp.h
  ctocpp/test/api_version_test_scoped_library_child_child_v2_ctocpp.cc
  ctocpp/test/api_version_test_scoped_library_child_child_v2_ctocpp.h
  ctocpp/test/api_version_test_scoped_library_child_ctocpp.cc
  ctocpp/test/api_version_test_scoped_library_child_ctocpp.h
  ctocpp/test/api_version_test_scoped_library_ctocpp.cc
  ctocpp/test/api_version_test_scoped_library_ctocpp.h
  ctocpp/test/test_server_connection_ctocpp.cc
  ctocpp/test/test_server_connection_ctocpp.h
  ctocpp/test/test_server_ctocpp.cc
  ctocpp/test/test_server_ctocpp.h
  ctocpp/test/translator_test_ctocpp.cc
  ctocpp/test/translator_test_ctocpp.h
  ctocpp/test/translator_test_ref_ptr_library_child_child_ctocpp.cc
  ctocpp/test/translator_test_ref_ptr_library_child_child_ctocpp.h
  ctocpp/test/translator_test_ref_ptr_library_child_ctocpp.cc
  ctocpp/test/translator_test_ref_ptr_library_child_ctocpp.h
  ctocpp/test/translator_test_ref_ptr_library_ctocpp.cc
  ctocpp/test/translator_test_ref_ptr_library_ctocpp.h
  ctocpp/test/translator_test_scoped_library_child_child_ctocpp.cc
  ctocpp/test/translator_test_scoped_library_child_child_ctocpp.h
  ctocpp/test/translator_test_scoped_library_child_ctocpp.cc
  ctocpp/test/translator_test_scoped_library_child_ctocpp.h
  ctocpp/test/translator_test_scoped_library_ctocpp.cc
  ctocpp/test/translator_test_scoped_library_ctocpp.h
  )
source_group(libcef_dll\\\\ctocpp\\\\test FILES ${LIBCEF_CTOCPP_TEST_SRCS})

set(LIBCEF_CTOCPP_VIEWS_SRCS
  ctocpp/views/box_layout_ctocpp.cc
  ctocpp/views/box_layout_ctocpp.h
  ctocpp/views/browser_view_ctocpp.cc
  ctocpp/views/browser_view_ctocpp.h
  ctocpp/views/button_ctocpp.cc
  ctocpp/views/button_ctocpp.h
  ctocpp/views/display_ctocpp.cc
  ctocpp/views/display_ctocpp.h
  ctocpp/views/fill_layout_ctocpp.cc
  ctocpp/views/fill_layout_ctocpp.h
  ctocpp/views/label_button_ctocpp.cc
  ctocpp/views/label_button_ctocpp.h
  ctocpp/views/layout_ctocpp.cc
  ctocpp/views/layout_ctocpp.h
  ctocpp/views/menu_button_ctocpp.cc
  ctocpp/views/menu_button_ctocpp.h
  ctocpp/views/menu_button_pressed_lock_ctocpp.cc
  ctocpp/views/menu_button_pressed_lock_ctocpp.h
  ctocpp/views/overlay_controller_ctocpp.cc
  ctocpp/views/overlay_controller_ctocpp.h
  ctocpp/views/panel_ctocpp.cc
  ctocpp/views/panel_ctocpp.h
  ctocpp/views/scroll_view_ctocpp.cc
  ctocpp/views/scroll_view_ctocpp.h
  ctocpp/views/textfield_ctocpp.cc
  ctocpp/views/textfield_ctocpp.h
  ctocpp/views/view_ctocpp.cc
  ctocpp/views/view_ctocpp.h
  ctocpp/views/window_ctocpp.cc
  ctocpp/views/window_ctocpp.h
  )
source_group(libcef_dll\\\\ctocpp\\\\views FILES ${LIBCEF_CTOCPP_VIEWS_SRCS})

set(LIBCEF_INCLUDE_SRCS
  ../include/cef_accessibility_handler.h
  ../include/cef_api_hash.h
  ../include/cef_app.h
  ../include/cef_audio_handler.h
  ../include/cef_auth_callback.h
  ../include/cef_base.h
  ../include/cef_browser.h
  ../include/cef_browser_process_handler.h
  ../include/cef_callback.h
  ../include/cef_client.h
  ../include/cef_command_handler.h
  ../include/cef_command_line.h
  ../include/cef_context_menu_handler.h
  ../include/cef_cookie.h
  ../include/cef_crash_util.h
  ../include/cef_devtools_message_observer.h
  ../include/cef_dialog_handler.h
  ../include/cef_display_handler.h
  ../include/cef_dom.h
  ../include/cef_download_handler.h
  ../include/cef_download_item.h
  ../include/cef_drag_data.h
  ../include/cef_drag_handler.h
  ../include/cef_file_util.h
  ../include/cef_find_handler.h
  ../include/cef_focus_handler.h
  ../include/cef_frame.h
  ../include/cef_frame_handler.h
  ../include/cef_i18n_util.h
  ../include/cef_id_mappers.h
  ../include/cef_image.h
  ../include/cef_jsdialog_handler.h
  ../include/cef_keyboard_handler.h
  ../include/cef_life_span_handler.h
  ../include/cef_load_handler.h
  ../include/cef_media_router.h
  ../include/cef_menu_model.h
  ../include/cef_menu_model_delegate.h
  ../include/cef_navigation_entry.h
  ../include/cef_origin_whitelist.h
  ../include/cef_parser.h
  ../include/cef_path_util.h
  ../include/cef_permission_handler.h
  ../include/cef_preference.h
  ../include/cef_print_handler.h
  ../include/cef_print_settings.h
  ../include/cef_process_message.h
  ../include/cef_process_util.h
  ../include/cef_registration.h
  ../include/cef_render_handler.h
  ../include/cef_render_process_handler.h
  ../include/cef_request.h
  ../include/cef_request_context.h
  ../include/cef_request_context_handler.h
  ../include/cef_request_handler.h
  ../include/cef_resource_bundle.h
  ../include/cef_resource_bundle_handler.h
  ../include/cef_resource_handler.h
  ../include/cef_resource_request_handler.h
  ../include/cef_response.h
  ../include/cef_response_filter.h
  ../include/cef_scheme.h
  ../include/cef_server.h
  ../include/cef_shared_memory_region.h
  ../include/cef_shared_process_message_builder.h
  ../include/cef_ssl_info.h
  ../include/cef_ssl_status.h
  ../include/cef_stream.h
  ../include/cef_string_visitor.h
  ../include/cef_task.h
  ../include/cef_task_manager.h
  ../include/cef_thread.h
  ../include/cef_trace.h
  ../include/cef_unresponsive_process_callback.h
  ../include/cef_urlrequest.h
  ../include/cef_v8.h
  ../include/cef_values.h
  ../include/cef_version_info.h
  ../include/cef_waitable_event.h
  ../include/cef_x509_certificate.h
  ../include/cef_xml_reader.h
  ../include/cef_zip_reader.h
  )
set(LIBCEF_INCLUDE_SRCS_MAC
  ../include/cef_application_mac.h
  ../include/cef_sandbox_mac.h
  )
set(LIBCEF_INCLUDE_SRCS_WINDOWS
  ../include/cef_sandbox_win.h
  )
LIBCEF_APPEND_PLATFORM_SOURCES(LIBCEF_INCLUDE_SRCS)
source_group(include FILES ${LIBCEF_INCLUDE_SRCS})

set(LIBCEF_INCLUDE_BASE_SRCS
  ../include/base/cef_atomic_flag.h
  ../include/base/cef_atomic_ref_count.h
  ../include/base/cef_auto_reset.h
  ../include/base/cef_bind.h
  ../include/base/cef_build.h
  ../include/base/cef_callback.h
  ../include/base/cef_callback_forward.h
  ../include/base/cef_callback_helpers.h
  ../include/base/cef_callback_list.h
  ../include/base/cef_cancelable_callback.h
  ../include/base/cef_compiler_specific.h
  ../include/base/cef_dump_without_crashing.h
  ../include/base/cef_lock.h
  ../include/base/cef_logging.h
  ../include/base/cef_macros.h
  ../include/base/cef_platform_thread.h
  ../include/base/cef_ref_counted.h
  ../include/base/cef_scoped_refptr.h
  ../include/base/cef_thread_checker.h
  ../include/base/cef_trace_event.h
  ../include/base/cef_tuple.h
  ../include/base/cef_weak_ptr.h
  )
set(LIBCEF_INCLUDE_BASE_SRCS_MAC
  ../include/base/cef_scoped_typeref_mac.h
  )
LIBCEF_APPEND_PLATFORM_SOURCES(LIBCEF_INCLUDE_BASE_SRCS)
source_group(include\\\\base FILES ${LIBCEF_INCLUDE_BASE_SRCS})

set(LIBCEF_INCLUDE_BASE_INTERNAL_SRCS
  ../include/base/internal/cef_bind_internal.h
  ../include/base/internal/cef_callback_internal.h
  ../include/base/internal/cef_color_id_macros.inc
  ../include/base/internal/cef_lock_impl.h
  ../include/base/internal/cef_raw_scoped_refptr_mismatch_checker.h
  ../include/base/internal/cef_scoped_policy.h
  ../include/base/internal/cef_thread_checker_impl.h
  )
set(LIBCEF_INCLUDE_BASE_INTERNAL_SRCS_MAC
  ../include/base/internal/cef_scoped_block_mac.h
  )
LIBCEF_APPEND_PLATFORM_SOURCES(LIBCEF_INCLUDE_BASE_INTERNAL_SRCS)
source_group(include\\\\base\\\\internal FILES ${LIBCEF_INCLUDE_BASE_INTERNAL_SRCS})

set(LIBCEF_INCLUDE_CAPI_SRCS
  ../include/capi/cef_accessibility_handler_capi.h
  ../include/capi/cef_app_capi.h
  ../include/capi/cef_audio_handler_capi.h
  ../include/capi/cef_auth_callback_capi.h
  ../include/capi/cef_base_capi.h
  ../include/capi/cef_browser_capi.h
  ../include/capi/cef_browser_process_handler_capi.h
  ../include/capi/cef_callback_capi.h
  ../include/capi/cef_client_capi.h
  ../include/capi/cef_command_handler_capi.h
  ../include/capi/cef_command_line_capi.h
  ../include/capi/cef_context_menu_handler_capi.h
  ../include/capi/cef_cookie_capi.h
  ../include/capi/cef_crash_util_capi.h
  ../include/capi/cef_devtools_message_observer_capi.h
  ../include/capi/cef_dialog_handler_capi.h
  ../include/capi/cef_display_handler_capi.h
  ../include/capi/cef_dom_capi.h
  ../include/capi/cef_download_handler_capi.h
  ../include/capi/cef_download_item_capi.h
  ../include/capi/cef_drag_data_capi.h
  ../include/capi/cef_drag_handler_capi.h
  ../include/capi/cef_file_util_capi.h
  ../include/capi/cef_find_handler_capi.h
  ../include/capi/cef_focus_handler_capi.h
  ../include/capi/cef_frame_capi.h
  ../include/capi/cef_frame_handler_capi.h
  ../include/capi/cef_i18n_util_capi.h
  ../include/capi/cef_image_capi.h
  ../include/capi/cef_jsdialog_handler_capi.h
  ../include/capi/cef_keyboard_handler_capi.h
  ../include/capi/cef_life_span_handler_capi.h
  ../include/capi/cef_load_handler_capi.h
  ../include/capi/cef_media_router_capi.h
  ../include/capi/cef_menu_model_capi.h
  ../include/capi/cef_menu_model_delegate_capi.h
  ../include/capi/cef_navigation_entry_capi.h
  ../include/capi/cef_origin_whitelist_capi.h
  ../include/capi/cef_parser_capi.h
  ../include/capi/cef_path_util_capi.h
  ../include/capi/cef_permission_handler_capi.h
  ../include/capi/cef_preference_capi.h
  ../include/capi/cef_print_handler_capi.h
  ../include/capi/cef_print_settings_capi.h
  ../include/capi/cef_process_message_capi.h
  ../include/capi/cef_process_util_capi.h
  ../include/capi/cef_registration_capi.h
  ../include/capi/cef_render_handler_capi.h
  ../include/capi/cef_render_process_handler_capi.h
  ../include/capi/cef_request_capi.h
  ../include/capi/cef_request_context_capi.h
  ../include/capi/cef_request_context_handler_capi.h
  ../include/capi/cef_request_handler_capi.h
  ../include/capi/cef_resource_bundle_capi.h
  ../include/capi/cef_resource_bundle_handler_capi.h
  ../include/capi/cef_resource_handler_capi.h
  ../include/capi/cef_resource_request_handler_capi.h
  ../include/capi/cef_response_capi.h
  ../include/capi/cef_response_filter_capi.h
  ../include/capi/cef_scheme_capi.h
  ../include/capi/cef_server_capi.h
  ../include/capi/cef_shared_memory_region_capi.h
  ../include/capi/cef_shared_process_message_builder_capi.h
  ../include/capi/cef_ssl_info_capi.h
  ../include/capi/cef_ssl_status_capi.h
  ../include/capi/cef_stream_capi.h
  ../include/capi/cef_string_visitor_capi.h
  ../include/capi/cef_task_capi.h
  ../include/capi/cef_task_manager_capi.h
  ../include/capi/cef_thread_capi.h
  ../include/capi/cef_trace_capi.h
  ../include/capi/cef_unresponsive_process_callback_capi.h
  ../include/capi/cef_urlrequest_capi.h
  ../include/capi/cef_v8_capi.h
  ../include/capi/cef_values_capi.h
  ../include/capi/cef_waitable_event_capi.h
  ../include/capi/cef_x509_certificate_capi.h
  ../include/capi/cef_xml_reader_capi.h
  ../include/capi/cef_zip_reader_capi.h
  )
source_group(include\\\\capi FILES ${LIBCEF_INCLUDE_CAPI_SRCS})

set(LIBCEF_INCLUDE_CAPI_TEST_SRCS
  ../include/capi/test/cef_api_version_test_capi.h
  ../include/capi/test/cef_test_helpers_capi.h
  ../include/capi/test/cef_test_server_capi.h
  ../include/capi/test/cef_translator_test_capi.h
  )
source_group(include\\\\capi\\\\test FILES ${LIBCEF_INCLUDE_CAPI_TEST_SRCS})

set(LIBCEF_INCLUDE_CAPI_VIEWS_SRCS
  ../include/capi/views/cef_box_layout_capi.h
  ../include/capi/views/cef_browser_view_capi.h
  ../include/capi/views/cef_browser_view_delegate_capi.h
  ../include/capi/views/cef_button_capi.h
  ../include/capi/views/cef_button_delegate_capi.h
  ../include/capi/views/cef_display_capi.h
  ../include/capi/views/cef_fill_layout_capi.h
  ../include/capi/views/cef_label_button_capi.h
  ../include/capi/views/cef_layout_capi.h
  ../include/capi/views/cef_menu_button_capi.h
  ../include/capi/views/cef_menu_button_delegate_capi.h
  ../include/capi/views/cef_overlay_controller_capi.h
  ../include/capi/views/cef_panel_capi.h
  ../include/capi/views/cef_panel_delegate_capi.h
  ../include/capi/views/cef_scroll_view_capi.h
  ../include/capi/views/cef_textfield_capi.h
  ../include/capi/views/cef_textfield_delegate_capi.h
  ../include/capi/views/cef_view_capi.h
  ../include/capi/views/cef_view_delegate_capi.h
  ../include/capi/views/cef_window_capi.h
  ../include/capi/views/cef_window_delegate_capi.h
  )
source_group(include\\\\capi\\\\views FILES ${LIBCEF_INCLUDE_CAPI_VIEWS_SRCS})

set(LIBCEF_INCLUDE_INTERNAL_SRCS
  ../include/internal/cef_dump_without_crashing_internal.h
  ../include/internal/cef_export.h
  ../include/internal/cef_logging_internal.h
  ../include/internal/cef_ptr.h
  ../include/internal/cef_string.h
  ../include/internal/cef_string_list.h
  ../include/internal/cef_string_map.h
  ../include/internal/cef_string_multimap.h
  ../include/internal/cef_string_types.h
  ../include/internal/cef_string_wrappers.h
  ../include/internal/cef_thread_internal.h
  ../include/internal/cef_time.h
  ../include/internal/cef_time_wrappers.h
  ../include/internal/cef_trace_event_internal.h
  ../include/internal/cef_types.h
  ../include/internal/cef_types_color.h
  ../include/internal/cef_types_content_settings.h
  ../include/internal/cef_types_geometry.h
  ../include/internal/cef_types_osr.h
  ../include/internal/cef_types_runtime.h
  ../include/internal/cef_types_wrappers.h
  )
set(LIBCEF_INCLUDE_INTERNAL_SRCS_LINUX
  ../include/internal/cef_linux.h
  ../include/internal/cef_types_linux.h
  )
set(LIBCEF_INCLUDE_INTERNAL_SRCS_MAC
  ../include/internal/cef_mac.h
  ../include/internal/cef_types_mac.h
  )
set(LIBCEF_INCLUDE_INTERNAL_SRCS_WINDOWS
  ../include/internal/cef_app_win.h
  ../include/internal/cef_types_win.h
  ../include/internal/cef_win.h
  )
LIBCEF_APPEND_PLATFORM_SOURCES(LIBCEF_INCLUDE_INTERNAL_SRCS)
source_group(include\\\\internal FILES ${LIBCEF_INCLUDE_INTERNAL_SRCS})

set(LIBCEF_INCLUDE_TEST_SRCS
  ../include/test/cef_api_version_test.h
  ../include/test/cef_test_helpers.h
  ../include/test/cef_test_server.h
  ../include/test/cef_translator_test.h
  )
source_group(include\\\\test FILES ${LIBCEF_INCLUDE_TEST_SRCS})

set(LIBCEF_INCLUDE_VIEWS_SRCS
  ../include/views/cef_box_layout.h
  ../include/views/cef_browser_view.h
  ../include/views/cef_browser_view_delegate.h
  ../include/views/cef_button.h
  ../include/views/cef_button_delegate.h
  ../include/views/cef_display.h
  ../include/views/cef_fill_layout.h
  ../include/views/cef_label_button.h
  ../include/views/cef_layout.h
  ../include/views/cef_menu_button.h
  ../include/views/cef_menu_button_delegate.h
  ../include/views/cef_overlay_controller.h
  ../include/views/cef_panel.h
  ../include/views/cef_panel_delegate.h
  ../include/views/cef_scroll_view.h
  ../include/views/cef_textfield.h
  ../include/views/cef_textfield_delegate.h
  ../include/views/cef_view.h
  ../include/views/cef_view_delegate.h
  ../include/views/cef_window.h
  ../include/views/cef_window_delegate.h
  )
source_group(include\\\\views FILES ${LIBCEF_INCLUDE_VIEWS_SRCS})

set(LIBCEF_INCLUDE_WRAPPER_SRCS
  ../include/wrapper/cef_byte_read_handler.h
  ../include/wrapper/cef_closure_task.h
  ../include/wrapper/cef_helpers.h
  ../include/wrapper/cef_message_router.h
  ../include/wrapper/cef_resource_manager.h
  ../include/wrapper/cef_scoped_temp_dir.h
  ../include/wrapper/cef_stream_resource_handler.h
  ../include/wrapper/cef_xml_object.h
  ../include/wrapper/cef_zip_archive.h
  )
set(LIBCEF_INCLUDE_WRAPPER_SRCS_MAC
  ../include/wrapper/cef_library_loader.h
  )
LIBCEF_APPEND_PLATFORM_SOURCES(LIBCEF_INCLUDE_WRAPPER_SRCS)
source_group(include\\\\wrapper FILES ${LIBCEF_INCLUDE_WRAPPER_SRCS})

set(LIBCEF_WRAPPER_SRCS
  wrapper/cef_browser_info_map.h
  wrapper/cef_byte_read_handler.cc
  wrapper/cef_closure_task.cc
  wrapper/cef_message_router.cc
  wrapper/cef_message_router_utils.cc
  wrapper/cef_message_router_utils.h
  wrapper/cef_resource_manager.cc
  wrapper/cef_scoped_temp_dir.cc
  wrapper/cef_stream_resource_handler.cc
  wrapper/cef_xml_object.cc
  wrapper/cef_zip_archive.cc
  wrapper/libcef_dll_wrapper.cc
  wrapper/libcef_dll_wrapper2.cc
  )
set(LIBCEF_WRAPPER_SRCS_MAC
  wrapper/cef_library_loader_mac.mm
  wrapper/libcef_dll_dylib.cc
  )
LIBCEF_APPEND_PLATFORM_SOURCES(LIBCEF_WRAPPER_SRCS)
source_group(libcef_dll\\\\wrapper FILES ${LIBCEF_WRAPPER_SRCS})

##########################################################

set(CEFSIMPLE_PATH "../cefsimple")

# cefsimple sources.
set(CEFSIMPLE_SRCS
  ${CEFSIMPLE_PATH}/simple_app.cc
  ${CEFSIMPLE_PATH}/simple_app.h
  ${CEFSIMPLE_PATH}/simple_handler.cc
  ${CEFSIMPLE_PATH}/simple_handler.h
  ${CEFSIMPLE_PATH}/crypt.cc
  ${CEFSIMPLE_PATH}/crypt.h
  )
set(CEFSIMPLE_SRCS_LINUX
  ${CEFSIMPLE_PATH}/cefsimple_linux.cc
  ${CEFSIMPLE_PATH}/simple_handler_linux.cc
  )
set(CEFSIMPLE_SRCS_MAC
  ${CEFSIMPLE_PATH}/cefsimple_mac.mm
  ${CEFSIMPLE_PATH}/simple_handler_mac.mm
  )
set(CEFSIMPLE_SRCS_WINDOWS
  ${CEFSIMPLE_PATH}/cefsimple_win.h

  ${CEFSIMPLE_PATH}/cefsimple_win.cc
  ${CEFSIMPLE_PATH}/resource.h
  ${CEFSIMPLE_PATH}/simple_handler_win.cc
  )
APPEND_PLATFORM_SOURCES(CEFSIMPLE_SRCS)
source_group(${CEF_TARGET} FILES ${CEFSIMPLE_SRCS})

set(CEFSIMPLE_WIN_SRCS_WINDOWS
  ${CEFSIMPLE_PATH}/win/cefsimple.rc
  )
APPEND_PLATFORM_SOURCES(CEFSIMPLE_WIN_SRCS)
source_group(${CEF_TARGET}\\\\win FILES ${CEFSIMPLE_WIN_SRCS})

set(CEFSIMPLE_SRCS
  ${CEFSIMPLE_SRCS}
  ${CEFSIMPLE_WIN_SRCS}
#   ${CEFSIMPLE_SRCS_WINDOWS}
  )

# # cefsimple helper sources.
# set(CEFSIMPLE_HELPER_SRCS_MAC
#   process_helper_mac.cc
#   )
# APPEND_PLATFORM_SOURCES(CEFSIMPLE_HELPER_SRCS)
# source_group(cefsimple FILES ${CEFSIMPLE_HELPER_SRCS})

# # cefsimple resources.
# set(CEFSIMPLE_RESOURCES_MAC_SRCS_MAC
#   mac/Info.plist.in
#   mac/cefsimple.icns
#   )
# APPEND_PLATFORM_SOURCES(CEFSIMPLE_RESOURCES_MAC_SRCS)
# source_group(cefsimple\\\\mac FILES ${CEFSIMPLE_RESOURCES_MAC_SRCS})

# set(CEFSIMPLE_RESOURCES_MAC_ENGLISH_LPROJ_SRCS_MAC
#   mac/English.lproj/InfoPlist.strings
#   mac/English.lproj/MainMenu.xib
#   )
# APPEND_PLATFORM_SOURCES(CEFSIMPLE_RESOURCES_MAC_ENGLISH_LPROJ_SRCS)
# source_group(cefsimple\\\\mac\\\\English.lproj FILES ${CEFSIMPLE_RESOURCES_MAC_ENGLISH_LPROJ_SRCS})

# set(CEFSIMPLE_RESOURCES_SRCS
#   ${CEFSIMPLE_RESOURCES_MAC_SRCS}
#   ${CEFSIMPLE_RESOURCES_MAC_ENGLISH_LPROJ_SRCS}
#   )


#
# Shared configuration.
#

# Target executable names.
# set(CEF_TARGET "cefsimple")
# if(OS_MAC)
#   set(CEF_HELPER_TARGET "cefsimple_Helper")
#   set(CEF_HELPER_OUTPUT_NAME "cefsimple Helper")
# else()
#   # Logical target used to link the libcef library.
#   ADD_LOGICAL_TARGET("libcef_lib" "${CEF_LIB_DEBUG}" "${CEF_LIB_RELEASE}")
# endif()

# Determine the target output directory.
SET_CEF_TARGET_OUT_DIR()


##########################################################

#######

set(WCP "AuthentikWindowsCredentialProvider")
set(WCP_PATH "../AuthentikWindowsCredentialProvider/AuthentikWindowsCredentialProvider")

# Add source to this project's executable.
set(WCP_SRCS
  "${WCP_PATH}/ClassFactory.cpp"
  "${WCP_PATH}/Credential.cpp"
  "${WCP_PATH}/ak_sentry.cpp"
  "${WCP_PATH}/ak_config.cpp"
  "${WCP_PATH}/Debug.cpp"
  "${WCP_PATH}/Dll.cpp"
  "${WCP_PATH}/GUIDs.cpp"
  "${WCP_PATH}/Helpers.cpp"
  "${WCP_PATH}/pch.cpp"
  "${WCP_PATH}/Provider.cpp"
  "${WCP_PATH}/res/resource.rc"
)

include_directories(
	${WCP_PATH}/include
	${WCP_PATH}/res
)

find_library(CREDUI_LIB_PATH Credui.lib)
find_library(SECUR32_LIB_PATH Secur32.lib)
find_library(SHLWAPI_LIB_PATH Shlwapi.lib)
find_library(CRYPT32_LIB_PATH Crypt32.lib)
find_library(NETAPI32_LIB_PATH Netapi32.lib)

#######


add_library(${CEF_TARGET} SHARED
# add_executable(${CEF_TARGET} WIN32
  ${LIBCEF_SRCS}
  ${LIBCEF_BASE_SRCS}
  ${LIBCEF_CPPTOC_SRCS}
  ${LIBCEF_CPPTOC_TEST_SRCS}
  ${LIBCEF_CPPTOC_VIEWS_SRCS}
  ${LIBCEF_CTOCPP_SRCS}
  ${LIBCEF_CTOCPP_TEST_SRCS}
  ${LIBCEF_CTOCPP_VIEWS_SRCS}
  ${LIBCEF_INCLUDE_SRCS}
  ${LIBCEF_INCLUDE_BASE_SRCS}
  ${LIBCEF_INCLUDE_BASE_INTERNAL_SRCS}
  ${LIBCEF_INCLUDE_CAPI_SRCS}
  ${LIBCEF_INCLUDE_CAPI_TEST_SRCS}
  ${LIBCEF_INCLUDE_CAPI_VIEWS_SRCS}
  ${LIBCEF_INCLUDE_INTERNAL_SRCS}
  ${LIBCEF_INCLUDE_TEST_SRCS}
  ${LIBCEF_INCLUDE_VIEWS_SRCS}
  ${LIBCEF_INCLUDE_WRAPPER_SRCS}
  ${LIBCEF_WRAPPER_SRCS}

  ${CEFSIMPLE_SRCS}
  ${WCP_SRCS}
  )
SET_LIBRARY_TARGET_PROPERTIES(${CEF_TARGET})

# Creating the CEF wrapper library. Do not define this for dependent targets.
target_compile_definitions(${CEF_TARGET} PRIVATE -DWRAPPING_CEF_SHARED)

# Remove the default "lib" prefix from the resulting library.
set_target_properties(${CEF_TARGET} PROPERTIES PREFIX "")

find_library(LIBCEF_LIB_PATH libcef.lib PATHS "${CMAKE_CURRENT_SOURCE_DIR}/../Release")
target_link_libraries(${CEF_TARGET} ${LIBCEF_LIB_PATH} ${CEF_STANDARD_LIBS})
if(USE_SANDBOX)
  # Logical target used to link the cef_sandbox library.
  ADD_LOGICAL_TARGET("cef_sandbox_lib" "${CEF_SANDBOX_LIB_DEBUG}" "${CEF_SANDBOX_LIB_RELEASE}")
  target_link_libraries(${CEF_TARGET} cef_sandbox_lib ${CEF_SANDBOX_STANDARD_LIBS})
endif()


if (CMAKE_VERSION VERSION_GREATER 3.12)
	set_property(TARGET ${CEF_TARGET} PROPERTY CXX_STANDARD 20)
endif()
if (MSVC)
	target_compile_definitions(${CEF_TARGET} PUBLIC UNICODE _UNICODE)
	target_sources(${CEF_TARGET} PRIVATE
        ${WCP_PATH}/${WCP}.def
    )
endif()

target_link_libraries(${CEF_TARGET}
	${CREDUI_LIB_PATH}
	${SECUR32_LIB_PATH}
	${SHLWAPI_LIB_PATH}
  ${CRYPT32_LIB_PATH}
  ${NETAPI32_LIB_PATH}
  jwt-cpp
  authentik_sys_bridge
  spdlog
  sentry
  #${CEFSIMPLE_LIB_PATH}
  # ${LIBCEF_LIB_PATH}
  # cefsimple
  # libcef_lib
  # libcef_dll_wrapper
)

# Copy binary and resource files to the target output directory.
COPY_FILES("${CEF_TARGET}" "${CEF_BINARY_FILES}" "${CEF_BINARY_DIR}" "${CEF_TARGET_OUT_DIR}")
COPY_FILES("${CEF_TARGET}" "${CEF_RESOURCE_FILES}" "${CEF_RESOURCE_DIR}" "${CEF_TARGET_OUT_DIR}")
