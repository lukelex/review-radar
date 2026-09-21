cmake_minimum_required(VERSION 3.21)

project(review-radar-linux VERSION 0.1.0 LANGUAGES CXX)

include(CTest)

find_package(Qt6 6.4 REQUIRED COMPONENTS Core DBus Gui Qml Quick)

qt_standard_project_setup(REQUIRES 6.4)

# `qt_policy` was introduced after the minimum supported Qt release.
if(COMMAND qt_policy)
    qt_policy(SET QTP0001 NEW)
    qt_policy(SET QTP0004 NEW)
endif()

qt_add_executable(review-radar-linux
    main.cpp
    queuecontroller.cpp
    queuecontroller.h
)

qt_add_qml_module(review-radar-linux
    URI ReviewRadar
    VERSION 1.0
    RESOURCE_PREFIX "/"
    QML_FILES qml/Main.qml
)

target_link_libraries(review-radar-linux PRIVATE Qt6::Core Qt6::DBus Qt6::Gui Qt6::Qml Qt6::Quick)

if(BUILD_TESTING)
    add_test(NAME review-radar-linux-starts COMMAND review-radar-linux --smoke-test)
    set_tests_properties(review-radar-linux-starts PROPERTIES
        ENVIRONMENT "QT_QPA_PLATFORM=offscreen"
    )
endif()

install(TARGETS review-radar-linux)
