cmake_minimum_required(VERSION 3.21)

project(review-radar-linux VERSION 0.1.0 LANGUAGES CXX)

include(GNUInstallDirs)

include(CTest)

find_package(Qt6 6.4 REQUIRED COMPONENTS Core DBus Gui Qml Quick QuickControls2 Widgets)

qt_standard_project_setup(REQUIRES 6.4)

# `qt_policy` was introduced after the minimum supported Qt release.
if(COMMAND qt_policy)
    qt_policy(SET QTP0001 NEW)
    qt_policy(SET QTP0004 NEW)
endif()

qt_add_executable(review-radar-linux
    main.cpp
    linuxosintegration.cpp
    linuxosintegration.h
    osintegration.h
    queuecontroller.cpp
    queuecontroller.h
)

set_source_files_properties(qml/Style.qml PROPERTIES QT_QML_SINGLETON_TYPE TRUE)

set(brand_assets "${CMAKE_CURRENT_SOURCE_DIR}/../../docs/assets")
qt_add_resources(review-radar-linux brand
    PREFIX "/assets" BASE "${brand_assets}" FILES "${brand_assets}/logo.svg"
)

set(workspace_qml qml/Main.qml qml/Style.qml qml/RadarButton.qml qml/Badge.qml
                   qml/PrCard.qml qml/DetailPanel.qml qml/ActivityTimeline.qml
                    qml/SkeletonCard.qml qml/ShortcutRow.qml qml/HealthSignal.qml qml/FrictionBadge.qml qml/RadarModal.qml qml/PreferencesDialog.qml)

qt_add_qml_module(review-radar-linux
    URI ReviewRadar
    VERSION 1.0
    RESOURCE_PREFIX "/"
    QML_FILES ${workspace_qml}
)

target_link_libraries(review-radar-linux PRIVATE Qt6::Core Qt6::DBus Qt6::Gui Qt6::Qml Qt6::Quick Qt6::QuickControls2 Qt6::Widgets)

if(BUILD_TESTING)
    find_package(Qt6 REQUIRED COMPONENTS Test)
    qt_add_executable(review-radar-ui-test tests/workspace.cpp linuxosintegration.cpp linuxosintegration.h osintegration.h queuecontroller.cpp queuecontroller.h)
    qt_add_resources(review-radar-ui-test brand
        PREFIX "/assets" BASE "${brand_assets}" FILES "${brand_assets}/logo.svg"
    )
    target_include_directories(review-radar-ui-test PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})
    target_link_libraries(review-radar-ui-test PRIVATE Qt6::Test Qt6::Core Qt6::DBus Qt6::Gui Qt6::Qml Qt6::Quick Qt6::QuickControls2 Qt6::Widgets)
    qt_add_qml_module(review-radar-ui-test
        URI ReviewRadarTest VERSION 1.0 RESOURCE_PREFIX "/"
        OUTPUT_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}/test-qml
        QML_FILES ${workspace_qml}
    )
    add_test(NAME review-radar-workspace COMMAND review-radar-ui-test)
    set_tests_properties(review-radar-workspace PROPERTIES
        ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software"
    )
    add_test(NAME review-radar-linux-starts COMMAND review-radar-linux --smoke-test)
    set_tests_properties(review-radar-linux-starts PROPERTIES
        ENVIRONMENT "QT_QPA_PLATFORM=offscreen"
    )
endif()

install(TARGETS review-radar-linux)
install(FILES ${CMAKE_CURRENT_SOURCE_DIR}/../../docs/assets/logo.svg
        DESTINATION ${CMAKE_INSTALL_DATADIR}/icons/hicolor/scalable/apps
        RENAME review-radar-linux.svg)
configure_file(review-radar-linux.desktop.in review-radar-linux.desktop @ONLY)
install(FILES ${CMAKE_CURRENT_BINARY_DIR}/review-radar-linux.desktop
        DESTINATION ${CMAKE_INSTALL_DATADIR}/applications)
