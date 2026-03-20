# Copyright The bngtester Authors
# Licensed under the GNU General Public License v3.0 or later.
# SPDX-License-Identifier: GPL-3.0-or-later

*** Comments ***
Validates that the shared entrypoint rejects invalid configuration
with correct error messages. All tests use --network none and run
synchronously — the container exits immediately on validation failure.

*** Settings ***
Library             OperatingSystem
Library             String
Resource            ../common.robot
Resource            ../subscriber.robot

Suite Teardown      Cleanup All Containers

*** Variables ***
${SUBSCRIBER_IMAGE}    veesixnetworks/bngtester:alpine-latest
${PREFIX}              bngt-val

*** Test Cases ***
Invalid ACCESS_METHOD
    ${name} =    Set Variable    ${PREFIX}-access-method
    ${rc}    ${output} =    Run Container    ${name}    ${SUBSCRIBER_IMAGE}
    ...    env=-e ACCESS_METHOD=invalid
    Should Not Be Equal As Integers    ${rc}    0
    Should Contain    ${output}    Invalid ACCESS_METHOD

Invalid ENCAP
    ${name} =    Set Variable    ${PREFIX}-encap
    ${rc}    ${output} =    Run Container    ${name}    ${SUBSCRIBER_IMAGE}
    ...    env=-e ENCAP=invalid
    Should Not Be Equal As Integers    ${rc}    0
    Should Contain    ${output}    Invalid ENCAP

Missing CVLAN For Single VLAN
    ${name} =    Set Variable    ${PREFIX}-cvlan
    ${rc}    ${output} =    Run Container    ${name}    ${SUBSCRIBER_IMAGE}
    ...    env=-e ENCAP=single
    Should Not Be Equal As Integers    ${rc}    0
    Should Contain    ${output}    CVLAN is required

Missing SVLAN For QinQ
    ${name} =    Set Variable    ${PREFIX}-svlan
    ${rc}    ${output} =    Run Container    ${name}    ${SUBSCRIBER_IMAGE}
    ...    env=-e ENCAP=qinq -e CVLAN=10
    Should Not Be Equal As Integers    ${rc}    0
    Should Contain    ${output}    SVLAN is required

Missing PPPOE_USER
    ${name} =    Set Variable    ${PREFIX}-pppoe-user
    ${rc}    ${output} =    Run Container    ${name}    ${SUBSCRIBER_IMAGE}
    ...    env=-e ACCESS_METHOD=pppoe
    Should Not Be Equal As Integers    ${rc}    0
    Should Contain    ${output}    PPPOE_USER is required

Missing PPPOE_PASSWORD
    ${name} =    Set Variable    ${PREFIX}-pppoe-pass
    ${rc}    ${output} =    Run Container    ${name}    ${SUBSCRIBER_IMAGE}
    ...    env=-e ACCESS_METHOD=pppoe -e PPPOE_USER=test
    Should Not Be Equal As Integers    ${rc}    0
    Should Contain    ${output}    PPPOE_PASSWORD is required

VLAN ID Out Of Range
    ${name} =    Set Variable    ${PREFIX}-vlan-range
    ${rc}    ${output} =    Run Container    ${name}    ${SUBSCRIBER_IMAGE}
    ...    env=-e ENCAP=single -e CVLAN=5000
    Should Not Be Equal As Integers    ${rc}    0
    Should Contain    ${output}    must be between 1 and 4094

Interface Name Too Long
    ${name} =    Set Variable    ${PREFIX}-ifname
    ${rc}    ${output} =    Run Container    ${name}    ${SUBSCRIBER_IMAGE}
    ...    env=-e PHYSICAL_IFACE=longifacename -e ENCAP=qinq -e SVLAN=100 -e CVLAN=10
    Should Not Be Equal As Integers    ${rc}    0
    Should Contain    ${output}    max 15

Missing NET_ADMIN Capability
    [Documentation]    VLAN creation requires NET_ADMIN. Without it, ip link add fails.
    ${name} =    Set Variable    ${PREFIX}-no-cap
    ${rc}    ${output} =    Run And Return Rc And Output
    ...    sudo docker run --rm --name ${name} --network none -e ENCAP=single -e CVLAN=100 ${SUBSCRIBER_IMAGE} 2>&1
    Should Not Be Equal As Integers    ${rc}    0
    Should Contain    ${output}    failed

*** Keywords ***
Cleanup All Containers
    FOR    ${suffix}    IN    access-method    encap    cvlan    svlan    pppoe-user    pppoe-pass    vlan-range    ifname    no-cap
        Remove Container    ${PREFIX}-${suffix}
    END
